//! High-level client-side wrappers for common objects and their events.

// libei has user_data for seat, device, etc. Do we need that?
// Want clarification in ei docs about events before/after done in some places?

// XXX device_for_interface may be empty if something is sent before done. Can any event be sent
// then?

// TODO: track last serial? Including destroyed event.
// - look at how libei handles it.

// This uses exhastive matching, so it will have to be updated when generated API is updated for
// any new events.

// WIP disconnected event? EOF?

#![allow(clippy::derive_partial_eq_without_eq)]

use crate::{ei, handshake::HandshakeResp, util, Error, Interface, Object, PendingRequestResult};
use std::{
    collections::{HashMap, VecDeque},
    fmt, io,
    os::unix::io::OwnedFd,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

#[derive(Debug)]
pub enum EventError {
    DeviceEventBeforeDone,
    DeviceSetupEventAfterDone,
    SeatSetupEventAfterDone,
    SeatEventBeforeDone,
    NoDeviceType,
    UnexpectedHandshakeEvent,
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::DeviceEventBeforeDone => write!(f, "device event before done"),
            Self::DeviceSetupEventAfterDone => write!(f, "device setup event after done"),
            Self::SeatSetupEventAfterDone => write!(f, "seat setup event after done"),
            Self::SeatEventBeforeDone => write!(f, "seat event before done"),
            Self::NoDeviceType => write!(f, "no device"),
            Self::UnexpectedHandshakeEvent => write!(f, "unexpected handshake event"),
        }
    }
}

impl std::error::Error for EventError {}

#[derive(Debug)]
struct ConnectionInner {
    context: ei::Context,
    handshake_resp: HandshakeResp,
    /// The last serial number used in an event by the server.
    serial: AtomicU32,
}

/// High-level client-side wrapper for `ei_connection`.
#[derive(Clone, Debug)]
pub struct Connection(Arc<ConnectionInner>);

impl Connection {
    /// Returns the interface proxy for the underlying `ei_connection` object.
    pub fn connection(&self) -> &ei::Connection {
        &self.0.handshake_resp.connection
    }

    /// Sends buffered messages. Call after you're finished with sending requests.
    pub fn flush(&self) -> rustix::io::Result<()> {
        self.0.context.flush()
    }

    fn update_serial(&self, value: u32) {
        self.0.serial.store(value, Ordering::Relaxed);
    }

    // TODO(axka, 2025-07-08): specify in the function name that this is the last serial from
    // the server, and not the client, and create a function for the other way around.
    /// Returns the last serial number used in an event by the server.
    pub fn serial(&self) -> u32 {
        self.0.serial.load(Ordering::Relaxed)
    }
}

/// Utility that converts low-level protocol-level events into high-level events defined in this
/// module.
pub struct EiEventConverter {
    pending_seats: HashMap<ei::Seat, SeatInner>,
    seats: HashMap<ei::Seat, Seat>,
    pending_devices: HashMap<ei::Device, DeviceInner>,
    devices: HashMap<ei::Device, Device>,
    device_for_interface: HashMap<Object, Device>,
    events: VecDeque<EiEvent>,
    pending_events: VecDeque<EiEvent>,
    callbacks: HashMap<ei::Callback, Box<dyn FnOnce(u64)>>,
    connection: Connection,
}

impl EiEventConverter {
    pub fn new(context: &ei::Context, handshake_resp: HandshakeResp) -> Self {
        Self {
            pending_seats: HashMap::new(),
            seats: HashMap::new(),
            pending_devices: HashMap::new(),
            devices: HashMap::new(),
            device_for_interface: HashMap::new(),
            events: VecDeque::new(),
            pending_events: VecDeque::new(),
            callbacks: HashMap::new(),
            connection: Connection(Arc::new(ConnectionInner {
                context: context.clone(),
                serial: AtomicU32::new(handshake_resp.serial),
                handshake_resp,
            })),
        }
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    // Based on behavior of `queue_event` in libei
    fn queue_event(&mut self, mut event: EiEvent) {
        if event.time_mut().is_some() {
            self.pending_events.push_back(event);
        } else if let EiEvent::Frame(Frame { time, .. }) = &event {
            if self.pending_events.is_empty() {
                return;
            }
            for mut pending_event in self.pending_events.drain(..) {
                *pending_event.time_mut().unwrap() = *time;
                self.events.push_back(pending_event);
            }
            self.events.push_back(event);
        } else {
            // TODO: If a device event, queue a frame if anything is pending
            self.events.push_back(event);
        }
    }

    pub fn handle_event(&mut self, event: ei::Event) -> Result<(), EventError> {
        match event {
            ei::Event::Handshake(_handshake, _event) => {
                return Err(EventError::UnexpectedHandshakeEvent);
            }
            ei::Event::Connection(_connection, event) => match event {
                ei::connection::Event::Seat { seat } => {
                    self.pending_seats.insert(
                        seat.clone(),
                        SeatInner {
                            seat,
                            name: None,
                            capabilities: HashMap::new(),
                        },
                    );
                }
                ei::connection::Event::Ping { ping } => {
                    ping.done(0);
                    if let Some(backend) = ping.0.backend() {
                        // XXX Error?
                        let _ = backend.flush();
                    }
                }
                ei::connection::Event::Disconnected {
                    last_serial,
                    reason,
                    explanation,
                } => {
                    self.queue_event(EiEvent::Disconnected(Disconnected {
                        last_serial,
                        reason,
                        explanation,
                    }));
                }
                ei::connection::Event::InvalidObject {
                    last_serial: _,
                    invalid_id: _,
                } => {
                    // TODO
                }
            },
            ei::Event::Callback(callback, event) => match event {
                ei::callback::Event::Done { callback_data } => {
                    if let Some(cb) = self.callbacks.remove(&callback) {
                        cb(callback_data);
                    }
                }
            },
            ei::Event::Pingpong(_ping_pong, event) => match event {},
            ei::Event::Seat(seat, event) => match event {
                ei::seat::Event::Name { name } => {
                    let seat = self
                        .pending_seats
                        .get_mut(&seat)
                        .ok_or(EventError::SeatSetupEventAfterDone)?;
                    seat.name = Some(name);
                }
                ei::seat::Event::Capability { mask, interface } => {
                    let seat = self
                        .pending_seats
                        .get_mut(&seat)
                        .ok_or(EventError::SeatSetupEventAfterDone)?;
                    seat.capabilities.insert(interface, mask);
                }
                ei::seat::Event::Done => {
                    let seat = self
                        .pending_seats
                        .remove(&seat)
                        .ok_or(EventError::SeatSetupEventAfterDone)?;
                    let seat = Seat(Arc::new(seat));
                    self.seats.insert(seat.0.seat.clone(), seat.clone());
                    self.queue_event(EiEvent::SeatAdded(SeatAdded { seat }));
                }
                ei::seat::Event::Device { device } => {
                    let seat = self
                        .seats
                        .get_mut(&seat)
                        .ok_or(EventError::SeatEventBeforeDone)?;
                    self.pending_devices.insert(
                        device.clone(),
                        DeviceInner {
                            device,
                            seat: seat.clone(),
                            name: None,
                            device_type: None,
                            interfaces: HashMap::new(),
                            dimensions: None,
                            regions: Vec::new(),
                            next_region_mapping_id: None,
                            keymap: None,
                        },
                    );
                }
                ei::seat::Event::Destroyed { serial } => {
                    self.connection.update_serial(serial);
                    self.pending_seats.remove(&seat);
                    if let Some(seat) = self.seats.remove(&seat) {
                        self.queue_event(EiEvent::SeatRemoved(SeatRemoved { seat }));
                    }
                }
            },
            ei::Event::Device(device, event) => match event {
                ei::device::Event::Name { name } => {
                    let device = self
                        .pending_devices
                        .get_mut(&device)
                        .ok_or(EventError::DeviceSetupEventAfterDone)?;
                    device.name = Some(name);
                }
                ei::device::Event::DeviceType { device_type } => {
                    let device = self
                        .pending_devices
                        .get_mut(&device)
                        .ok_or(EventError::DeviceSetupEventAfterDone)?;
                    device.device_type = Some(device_type);
                }
                ei::device::Event::Interface { object } => {
                    let device = self
                        .pending_devices
                        .get_mut(&device)
                        .ok_or(EventError::DeviceSetupEventAfterDone)?;
                    device
                        .interfaces
                        .insert(object.interface().to_string(), object);
                }
                ei::device::Event::Dimensions { width, height } => {
                    let device = self
                        .pending_devices
                        .get_mut(&device)
                        .ok_or(EventError::DeviceSetupEventAfterDone)?;
                    device.dimensions = Some((width, height));
                }
                ei::device::Event::Region {
                    offset_x,
                    offset_y,
                    width,
                    hight,
                    scale,
                } => {
                    let device = self
                        .pending_devices
                        .get_mut(&device)
                        .ok_or(EventError::DeviceSetupEventAfterDone)?;
                    device.regions.push(Region {
                        x: offset_x,
                        y: offset_y,
                        width,
                        height: hight,
                        scale,
                        mapping_id: device.next_region_mapping_id.clone(),
                    });
                }
                ei::device::Event::Done => {
                    let device = self
                        .pending_devices
                        .remove(&device)
                        .ok_or(EventError::DeviceSetupEventAfterDone)?;
                    if device.device_type.is_none() {
                        return Err(EventError::NoDeviceType);
                    }
                    let device = Device(Arc::new(device));
                    self.devices.insert(device.0.device.clone(), device.clone());
                    for i in device.0.interfaces.values() {
                        self.device_for_interface.insert(i.clone(), device.clone());
                    }
                    self.queue_event(EiEvent::DeviceAdded(DeviceAdded { device }));
                }
                ei::device::Event::Resumed { serial } => {
                    self.connection.update_serial(serial);
                    let device = self
                        .devices
                        .get(&device)
                        .ok_or(EventError::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::DeviceResumed(DeviceResumed {
                        device: device.clone(),
                        serial,
                    }));
                }
                ei::device::Event::Paused { serial } => {
                    self.connection.update_serial(serial);
                    let device = self
                        .devices
                        .get(&device)
                        .ok_or(EventError::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::DevicePaused(DevicePaused {
                        device: device.clone(),
                        serial,
                    }));
                }
                ei::device::Event::StartEmulating { serial, sequence } => {
                    self.connection.update_serial(serial);
                    let device = self
                        .devices
                        .get(&device)
                        .ok_or(EventError::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::DeviceStartEmulating(DeviceStartEmulating {
                        device: device.clone(),
                        serial,
                        sequence,
                    }));
                }
                ei::device::Event::StopEmulating { serial } => {
                    self.connection.update_serial(serial);
                    let device = self
                        .devices
                        .get(&device)
                        .ok_or(EventError::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::DeviceStopEmulating(DeviceStopEmulating {
                        device: device.clone(),
                        serial,
                    }));
                }
                ei::device::Event::RegionMappingId { mapping_id } => {
                    let device = self
                        .pending_devices
                        .get_mut(&device)
                        .ok_or(EventError::DeviceEventBeforeDone)?;
                    device.next_region_mapping_id = Some(mapping_id);
                }
                ei::device::Event::Frame { serial, timestamp } => {
                    self.connection.update_serial(serial);
                    let device = self
                        .devices
                        .get(&device)
                        .ok_or(EventError::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::Frame(Frame {
                        device: device.clone(),
                        serial,
                        time: timestamp,
                    }));
                }
                ei::device::Event::Destroyed { serial } => {
                    self.connection.update_serial(serial);
                    self.pending_devices.remove(&device);
                    if let Some(device) = self.devices.remove(&device) {
                        self.queue_event(EiEvent::DeviceRemoved(DeviceRemoved { device }));
                    }
                }
            },
            ei::Event::Keyboard(keyboard, event) => match event {
                ei::keyboard::Event::Keymap {
                    keymap_type,
                    size,
                    keymap,
                } => {
                    let device = self
                        .pending_devices
                        .values_mut()
                        .find(|i| i.interfaces.values().any(|j| j == &keyboard.0))
                        .ok_or(EventError::DeviceSetupEventAfterDone)?;
                    device.keymap = Some(Keymap {
                        type_: keymap_type,
                        size,
                        fd: keymap,
                    });
                }
                ei::keyboard::Event::Key { key, state } => {
                    let device = self
                        .device_for_interface
                        .get(&keyboard.0)
                        .ok_or(EventError::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::KeyboardKey(KeyboardKey {
                        device: device.clone(),
                        time: 0,
                        key,
                        state,
                    }));
                }
                ei::keyboard::Event::Modifiers {
                    serial,
                    depressed,
                    locked,
                    latched,
                    group,
                } => {
                    self.connection.update_serial(serial);
                    let device = self
                        .device_for_interface
                        .get(&keyboard.0)
                        .ok_or(EventError::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::KeyboardModifiers(KeyboardModifiers {
                        device: device.clone(),
                        serial,
                        depressed,
                        locked,
                        latched,
                        group,
                    }));
                }
                ei::keyboard::Event::Destroyed { serial } => {
                    self.connection.update_serial(serial);
                    // TODO does interface need to be removed from `Device`?
                    self.device_for_interface.remove(&keyboard.0);
                }
            },
            ei::Event::Pointer(pointer, event) => {
                let device = self
                    .device_for_interface
                    .get(&pointer.0)
                    .ok_or(EventError::DeviceEventBeforeDone)?;
                match event {
                    ei::pointer::Event::MotionRelative { x, y } => {
                        self.queue_event(EiEvent::PointerMotion(PointerMotion {
                            device: device.clone(),
                            time: 0,
                            dx: x,
                            dy: y,
                        }));
                    }
                    ei::pointer::Event::Destroyed { serial } => {
                        self.connection.update_serial(serial);
                        // TODO does interface need to be removed from `Device`?
                        self.device_for_interface.remove(&pointer.0);
                    }
                }
            }
            ei::Event::PointerAbsolute(pointer_absolute, event) => {
                let device = self
                    .device_for_interface
                    .get(&pointer_absolute.0)
                    .ok_or(EventError::DeviceEventBeforeDone)?;
                match event {
                    ei::pointer_absolute::Event::MotionAbsolute { x, y } => {
                        self.queue_event(EiEvent::PointerMotionAbsolute(PointerMotionAbsolute {
                            device: device.clone(),
                            time: 0,
                            dx_absolute: x,
                            dy_absolute: y,
                        }));
                    }
                    ei::pointer_absolute::Event::Destroyed { serial } => {
                        self.connection.update_serial(serial);
                        // TODO does interface need to be removed from `Device`?
                        self.device_for_interface.remove(&pointer_absolute.0);
                    }
                }
            }
            ei::Event::Scroll(scroll, event) => {
                let device = self
                    .device_for_interface
                    .get(&scroll.0)
                    .ok_or(EventError::DeviceEventBeforeDone)?;
                match event {
                    ei::scroll::Event::Scroll { x, y } => {
                        self.queue_event(EiEvent::ScrollDelta(ScrollDelta {
                            device: device.clone(),
                            time: 0,
                            dx: x,
                            dy: y,
                        }));
                    }
                    ei::scroll::Event::ScrollDiscrete { x, y } => {
                        self.queue_event(EiEvent::ScrollDiscrete(ScrollDiscrete {
                            device: device.clone(),
                            time: 0,
                            discrete_dx: x,
                            discrete_dy: y,
                        }));
                    }
                    ei::scroll::Event::ScrollStop { x, y, is_cancel } => {
                        if is_cancel != 0 {
                            self.queue_event(EiEvent::ScrollCancel(ScrollCancel {
                                device: device.clone(),
                                time: 0,
                                x: x != 0,
                                y: y != 0,
                            }));
                        } else {
                            self.queue_event(EiEvent::ScrollStop(ScrollStop {
                                device: device.clone(),
                                time: 0,
                                x: x != 0,
                                y: y != 0,
                            }));
                        }
                    }
                    ei::scroll::Event::Destroyed { serial } => {
                        self.connection.update_serial(serial);
                        // TODO does interface need to be removed from `Device`?
                        self.device_for_interface.remove(&scroll.0);
                    }
                }
            }
            ei::Event::Button(button, event) => {
                let device = self
                    .device_for_interface
                    .get(&button.0)
                    .ok_or(EventError::DeviceEventBeforeDone)?;
                match event {
                    ei::button::Event::Button { button, state } => {
                        self.queue_event(EiEvent::Button(Button {
                            device: device.clone(),
                            time: 0,
                            button,
                            state,
                        }));
                    }
                    ei::button::Event::Destroyed { serial } => {
                        self.connection.update_serial(serial);
                        // TODO does interface need to be removed from `Device`?
                        self.device_for_interface.remove(&button.0);
                    }
                }
            }
            ei::Event::Touchscreen(touchscreen, event) => {
                let device = self
                    .device_for_interface
                    .get(&touchscreen.0)
                    .ok_or(EventError::DeviceEventBeforeDone)?;
                match event {
                    ei::touchscreen::Event::Down { touchid, x, y } => {
                        self.queue_event(EiEvent::TouchDown(TouchDown {
                            device: device.clone(),
                            time: 0,
                            touch_id: touchid,
                            x,
                            y,
                        }));
                    }
                    ei::touchscreen::Event::Motion { touchid, x, y } => {
                        self.queue_event(EiEvent::TouchMotion(TouchMotion {
                            device: device.clone(),
                            time: 0,
                            touch_id: touchid,
                            x,
                            y,
                        }));
                    }
                    ei::touchscreen::Event::Up { touchid } => {
                        self.queue_event(EiEvent::TouchUp(TouchUp {
                            device: device.clone(),
                            time: 0,
                            touch_id: touchid,
                        }));
                    }
                    ei::touchscreen::Event::Cancel { touchid } => {
                        self.queue_event(EiEvent::TouchCancel(TouchCancel {
                            device: device.clone(),
                            time: 0,
                            touch_id: touchid,
                        }));
                    }
                    ei::touchscreen::Event::Destroyed { serial } => {
                        self.connection.update_serial(serial);
                        // TODO does interface need to be removed from `Device`?
                        self.device_for_interface.remove(&touchscreen.0);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn next_event(&mut self) -> Option<EiEvent> {
        self.events.pop_front()
    }

    pub fn add_callback_handler<F: FnOnce(u64) + 'static>(
        &mut self,
        callback: ei::Callback,
        cb: F,
    ) {
        self.callbacks.insert(callback, Box::new(cb));
    }
}

/// A [`Region`] defines the area that is accessible by a device.
#[derive(Debug)]
pub struct Region {
    /// X offset in logical pixels.
    pub x: u32,
    /// Y offset in logical pixels.
    pub y: u32,
    /// Width in logical pixels.
    pub width: u32,
    /// Height in logical pixels.
    pub height: u32,
    /// Physical scale for this region.
    pub scale: f32,
    /// ID that can be used by the client to, for example, match a video stream with a region.
    pub mapping_id: Option<String>,
}

/// A keymap from the server.
#[derive(Debug)]
pub struct Keymap {
    /// File descriptor to the keymap.
    pub fd: OwnedFd,
    /// The size of the keymap in bytes.
    pub size: u32,
    /// Keymap type.
    pub type_: ei::keyboard::KeymapType,
}

/// A capability of a seat used when advertising seats and binding to capabilities in seats.
// TODO: bitflags?
// TODO(axka, 2025-07-08): rename to SeatCapability?
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(u64)]
pub enum DeviceCapability {
    Pointer,
    PointerAbsolute,
    Keyboard,
    Touch,
    Scroll,
    Button,
}

impl DeviceCapability {
    /// Returns the name of the interface for this capability.
    pub(crate) fn name(self) -> &'static str {
        match self {
            DeviceCapability::Pointer => ei::Pointer::NAME,
            DeviceCapability::PointerAbsolute => ei::PointerAbsolute::NAME,
            DeviceCapability::Keyboard => ei::Keyboard::NAME,
            DeviceCapability::Touch => ei::Touchscreen::NAME,
            DeviceCapability::Scroll => ei::Scroll::NAME,
            DeviceCapability::Button => ei::Button::NAME,
        }
    }
}

struct SeatInner {
    seat: ei::Seat,
    name: Option<String>,
    capabilities: HashMap<String, u64>,
}

/// High-level client-side wrapper for `ei_seat`.
#[derive(Clone)]
pub struct Seat(Arc<SeatInner>);

impl fmt::Debug for Seat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = self.name() {
            write!(f, "Seat(\"{name}\")")
        } else {
            write!(f, "Seat(None)")
        }
    }
}

impl PartialEq for Seat {
    fn eq(&self, rhs: &Seat) -> bool {
        Arc::ptr_eq(&self.0, &rhs.0)
    }
}

impl Eq for Seat {}

impl std::hash::Hash for Seat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.seat.0.id().hash(state);
    }
}

impl Seat {
    /// Returns the name of the seat, as provided by the server.
    pub fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    // TODO has_capability

    pub fn bind_capabilities(&self, capabilities: &[DeviceCapability]) {
        let mut caps = 0;
        for i in capabilities {
            if let Some(value) = self.0.capabilities.get(i.name()) {
                caps |= value;
            }
        }
        self.0.seat.bind(caps);
    }

    // TODO: mirror C API more?
    // fn unbind_capabilities() {}
}

struct DeviceInner {
    device: ei::Device,
    seat: Seat,
    name: Option<String>,
    device_type: Option<ei::device::DeviceType>,
    interfaces: HashMap<String, crate::Object>,
    dimensions: Option<(u32, u32)>,
    regions: Vec<Region>,
    // Only used before `done`
    next_region_mapping_id: Option<String>,
    // Only defined device with `ei_keyboard` interface
    keymap: Option<Keymap>,
}

/// High-level client-side wrapper for `ei_device`.
#[derive(Clone)]
pub struct Device(Arc<DeviceInner>);

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = self.name() {
            write!(f, "Device(\"{name}\")")
        } else {
            write!(f, "Device(None)")
        }
    }
}

impl Device {
    /// Returns the high-level [`Seat`] wrapper for the device.
    pub fn seat(&self) -> &Seat {
        &self.0.seat
    }

    /// Returns the interface proxy for the underlying `ei_device` object.
    pub fn device(&self) -> &ei::Device {
        &self.0.device
    }

    /// Returns the name of the device, as provided by the server.
    pub fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    /// Returns the device's type.
    pub fn device_type(&self) -> ei::device::DeviceType {
        self.0.device_type.unwrap()
    }

    /// Returns the device's dimensions, if applicable.
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.0.dimensions
    }

    /// Returns the device's regions.
    pub fn regions(&self) -> &[Region] {
        &self.0.regions
    }

    /// Returns the device's keymap, if applicable.
    pub fn keymap(&self) -> Option<&Keymap> {
        self.0.keymap.as_ref()
    }

    /// Returns an interface proxy if it is implemented for this device.
    ///
    /// Interfaces of devices are implemented, such that there is one `ei_device` object and other objects (for example `ei_keyboard`) denoting capabilities.
    pub fn interface<T: ei::Interface>(&self) -> Option<T> {
        self.0.interfaces.get(T::NAME)?.clone().downcast()
    }

    /// Returns `true` if this device has an interface matching the provided capability.
    pub fn has_capability(&self, capability: DeviceCapability) -> bool {
        self.0.interfaces.contains_key(capability.name())
    }
}

impl PartialEq for Device {
    fn eq(&self, rhs: &Device) -> bool {
        Arc::ptr_eq(&self.0, &rhs.0)
    }
}

impl Eq for Device {}

impl std::hash::Hash for Device {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.device.0.id().hash(state);
    }
}

/// Enum containing all possible events the high-level utilities will give for a client implementation to handle.
#[derive(Clone, Debug, PartialEq)]
pub enum EiEvent {
    // Connected,
    Disconnected(Disconnected),
    SeatAdded(SeatAdded),
    SeatRemoved(SeatRemoved),
    DeviceAdded(DeviceAdded),
    DeviceRemoved(DeviceRemoved),
    DevicePaused(DevicePaused),
    DeviceResumed(DeviceResumed),
    KeyboardModifiers(KeyboardModifiers),
    // Only for reciever context
    Frame(Frame),
    DeviceStartEmulating(DeviceStartEmulating),
    DeviceStopEmulating(DeviceStopEmulating),
    PointerMotion(PointerMotion),
    PointerMotionAbsolute(PointerMotionAbsolute),
    Button(Button),
    ScrollDelta(ScrollDelta),
    ScrollStop(ScrollStop),
    ScrollCancel(ScrollCancel),
    ScrollDiscrete(ScrollDiscrete),
    KeyboardKey(KeyboardKey),
    TouchDown(TouchDown),
    TouchUp(TouchUp),
    TouchMotion(TouchMotion),
    TouchCancel(TouchCancel),
}

impl EiEvent {
    // Events that are grouped by frames need their times set when the
    // frame event occurs.
    /// Returns the `time` property of this event, if applicable.
    fn time_mut(&mut self) -> Option<&mut u64> {
        match self {
            Self::PointerMotion(evt) => Some(&mut evt.time),
            Self::PointerMotionAbsolute(evt) => Some(&mut evt.time),
            Self::Button(evt) => Some(&mut evt.time),
            Self::ScrollDelta(evt) => Some(&mut evt.time),
            Self::ScrollStop(evt) => Some(&mut evt.time),
            Self::ScrollCancel(evt) => Some(&mut evt.time),
            Self::ScrollDiscrete(evt) => Some(&mut evt.time),
            Self::KeyboardKey(evt) => Some(&mut evt.time),
            Self::TouchDown(evt) => Some(&mut evt.time),
            Self::TouchUp(evt) => Some(&mut evt.time),
            Self::TouchMotion(evt) => Some(&mut evt.time),
            Self::TouchCancel(evt) => Some(&mut evt.time),
            Self::Disconnected(_)
            | Self::SeatAdded(_)
            | Self::SeatRemoved(_)
            | Self::DeviceAdded(_)
            | Self::DeviceRemoved(_)
            | Self::DevicePaused(_)
            | Self::DeviceResumed(_)
            | Self::KeyboardModifiers(_)
            | Self::Frame(_)
            | Self::DeviceStartEmulating(_)
            | Self::DeviceStopEmulating(_) => None,
        }
    }
}

/// High-level translation of [`ei_connection.disconnected`](ei::connection::Event::Disconnected).
#[derive(Clone, Debug, PartialEq)]
pub struct Disconnected {
    pub last_serial: u32,
    pub reason: ei::connection::DisconnectReason,
    pub explanation: String,
}

/// High-level translation of the seat description events ending with [`ei_seat.done`](ei::seat::Event::Done).
#[derive(Clone, Debug, PartialEq)]
pub struct SeatAdded {
    pub seat: Seat,
}

/// High-level translation of [`ei_seat.destroyed`](ei::seat::Event::Destroyed).
///
/// The seat was removed from the tracking data structure, and this is the last time it will appear
/// in an event.
#[derive(Clone, Debug, PartialEq)]
pub struct SeatRemoved {
    pub seat: Seat,
}

/// High-level translation of events leading up to [`ei_device.done`](ei::device::Event::Done).
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceAdded {
    /// High-level [`Device`] wrapper.
    pub device: Device,
}

/// High-level translation of [`ei_device.destroyed`](ei::device::Event::Destroyed).
///
/// The device was removed from the tracking data structure, and this is the last time it will
/// appear in an event.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceRemoved {
    /// High-level [`Device`] wrapper.
    pub device: Device,
}

/// High-level translation of [`ei_device.paused`](ei::device::Event::Paused).
#[derive(Clone, Debug, PartialEq)]
pub struct DevicePaused {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    pub serial: u32,
}

/// High-level translation of [`ei_device.resumed`](ei::device::Event::Resumed).
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceResumed {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    pub serial: u32,
}

/// High-level translation of [`ei_keyboard.modifiers`](ei::keyboard::Event::Modifiers).
#[derive(Clone, Debug, PartialEq)]
pub struct KeyboardModifiers {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    pub serial: u32,
    pub depressed: u32,
    pub latched: u32,
    pub locked: u32,
    pub group: u32,
}

/// High-level translation of [`ei_device.frame`](ei::device::Event::Frame).
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    pub serial: u32,
    /// Timestamp in microseconds.
    pub time: u64,
}

/// High-level translation of [`ei_device.start_emulating`](ei::device::Event::StartEmulating).
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStartEmulating {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    pub serial: u32,
    pub sequence: u32,
}

/// High-level translation of [`ei_device.stop_emulating`](ei::device::Event::StopEmulating).
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStopEmulating {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    pub serial: u32,
}

/// High-level translation of [`ei_pointer.motion_relative`](ei::pointer::Event::MotionRelative).
#[derive(Clone, Debug, PartialEq)]
pub struct PointerMotion {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub dx: f32,
    pub dy: f32,
}

/// High-level translation of [`ei_pointer_absolute.motion_absolute`](ei::pointer_absolute::Event::MotionAbsolute).
#[derive(Clone, Debug, PartialEq)]
pub struct PointerMotionAbsolute {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub dx_absolute: f32,
    pub dy_absolute: f32,
}

/// High-level translation of [`ei_button.button`](ei::button::Event::Button).
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub button: u32,
    pub state: ei::button::ButtonState,
}

/// High-level translation of [`ei_scroll.scroll`](ei::scroll::Event::Scroll).
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDelta {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub dx: f32,
    pub dy: f32,
}

/// High-level translation of [`ei_scroll.scroll_stop`](ei::scroll::Event::ScrollStop) when its `is_cancel` is zero.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollStop {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub x: bool,
    pub y: bool,
}

/// High-level translation of [`ei_scroll.scroll_stop`](ei::scroll::Event::ScrollStop) when its `is_cancel` is nonzero.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollCancel {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub x: bool,
    pub y: bool,
}

/// High-level translation of [`ei_scroll.scroll_discrete`](ei::scroll::Event::ScrollDiscrete).
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDiscrete {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub discrete_dx: i32,
    pub discrete_dy: i32,
}

/// High-level translation of [`ei_keyboard.key`](ei::keyboard::Event::Key).
#[derive(Clone, Debug, PartialEq)]
pub struct KeyboardKey {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Key code (according to the current keymap, if any).
    pub key: u32,
    /// Logical key state.
    pub state: ei::keyboard::KeyState,
}

/// High-level translation of [`ei_touchscreen.down`](ei::touchscreen::Event::Down).
#[derive(Clone, Debug, PartialEq)]
pub struct TouchDown {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Unique touch ID, defined in this event.
    pub touch_id: u32,
    pub x: f32,
    pub y: f32,
}

/// High-level translation of [`ei_touchscreen.motion`](ei::touchscreen::Event::Motion).
#[derive(Clone, Debug, PartialEq)]
pub struct TouchMotion {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Unique touch ID, defined in [`TouchDown`].
    pub touch_id: u32,
    pub x: f32,
    pub y: f32,
}

/// High-level translation of [`ei_touchscreen.up`](ei::touchscreen::Event::Up).
#[derive(Clone, Debug, PartialEq)]
pub struct TouchUp {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Unique touch ID, defined in [`TouchDown`]. It may be reused after this event.
    pub touch_id: u32,
}

/// High-level translation of [`ei_touchscreen.chcancel`](ei::touchscreen::Event::Cancel).
#[derive(Clone, Debug, PartialEq)]
pub struct TouchCancel {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Unique touch ID, defined in [`TouchDown`].
    pub touch_id: u32,
}

/// Trait marking events that happen on a seat.
pub trait SeatEvent {
    /// Returns the high-level [`Seat`] wrapper for this event.
    fn seat(&self) -> &Seat;
}

/// Trait marking events that happen on a device.
pub trait DeviceEvent: SeatEvent {
    /// Returns the high-level [`Device`] wrapper for this event.
    fn device(&self) -> &Device;
}

/// Trait marking events that have microsecond-precision timestamps.
pub trait EventTime: DeviceEvent {
    /// Returns the timestamp in microseconds of `CLOCK_MONOTONIC`.
    fn time(&self) -> u64;
}

impl<T: DeviceEvent> SeatEvent for T {
    fn seat(&self) -> &Seat {
        &self.device().0.seat
    }
}

impl SeatEvent for SeatAdded {
    fn seat(&self) -> &Seat {
        &self.seat
    }
}

impl SeatEvent for SeatRemoved {
    fn seat(&self) -> &Seat {
        &self.seat
    }
}

macro_rules! impl_device_trait {
    ($ty:ty) => {
        impl DeviceEvent for $ty {
            fn device(&self) -> &Device {
                &self.device
            }
        }
    };

    ($ty:ty; time) => {
        impl_device_trait!($ty);

        impl EventTime for $ty {
            fn time(&self) -> u64 {
                self.time
            }
        }
    };
}

impl_device_trait!(DeviceAdded);
impl_device_trait!(DeviceRemoved);
impl_device_trait!(DevicePaused);
impl_device_trait!(DeviceResumed);
impl_device_trait!(KeyboardModifiers);

impl_device_trait!(Frame; time);
impl_device_trait!(DeviceStartEmulating);
impl_device_trait!(DeviceStopEmulating);
impl_device_trait!(PointerMotion; time);
impl_device_trait!(PointerMotionAbsolute; time);
impl_device_trait!(Button; time);
impl_device_trait!(ScrollDelta; time);
impl_device_trait!(ScrollStop; time);
impl_device_trait!(ScrollCancel; time);
impl_device_trait!(ScrollDiscrete; time);
impl_device_trait!(KeyboardKey; time);
impl_device_trait!(TouchDown; time);
impl_device_trait!(TouchUp; time);
impl_device_trait!(TouchMotion; time);
impl_device_trait!(TouchCancel; time);

/// An iterator that uses [`EiEventConverter`] to convert low-level protocol-level events into
/// high-level events defined in this module.
pub struct EiConvertEventIterator {
    context: ei::Context,
    converter: EiEventConverter,
}

impl EiConvertEventIterator {
    fn new(context: ei::Context, handshake_resp: HandshakeResp) -> Self {
        Self {
            converter: EiEventConverter::new(&context, handshake_resp),
            context,
        }
    }
}

impl Iterator for EiConvertEventIterator {
    type Item = Result<crate::event::EiEvent, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(event) = self.converter.next_event() {
                return Some(Ok(event));
            }
            if let Err(err) = util::poll_readable(&self.context) {
                return Some(Err(err.into()));
            }
            match self.context.read() {
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return None,
                Err(err) => return Some(Err(err.into())),
                Ok(_) => {}
            };
            while let Some(result) = self.context.pending_event() {
                let request = match result {
                    PendingRequestResult::Request(request) => request,
                    PendingRequestResult::ParseError(parse_error) => {
                        return Some(Err(parse_error.into()))
                    }
                    PendingRequestResult::InvalidObject(_invalid_object) => {
                        // Log?
                        continue;
                    }
                };

                if let Err(err) = self.converter.handle_event(request) {
                    return Some(Err(err.into()));
                }
            }
        }
    }
}

impl ei::Context {
    pub fn handshake_blocking(
        &self,
        name: &str,
        context_type: ei::handshake::ContextType,
    ) -> Result<(crate::event::Connection, EiConvertEventIterator), Error> {
        let resp = crate::handshake::ei_handshake_blocking(self, name, context_type)?;
        let iterator = EiConvertEventIterator::new(self.clone(), resp);
        let connection = iterator.converter.connection().clone();
        Ok((connection, iterator))
    }
}
