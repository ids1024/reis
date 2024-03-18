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

use crate::{ei, util, Interface, Object, ParseError, PendingRequestResult};
use std::{
    collections::{HashMap, VecDeque},
    fmt, io,
    os::unix::io::OwnedFd,
    sync::Arc,
};

// struct ReceiverStream(EiEventStream, ());

#[derive(Debug)]
pub enum Error {
    DeviceEventBeforeDone,
    DeviceSetupEventAfterDone,
    SeatSetupEventAfterDone,
    SeatEventBeforeDone,
    NoDeviceType,
    UnexpectedHandshakeEvent,
}

impl fmt::Display for Error {
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

impl std::error::Error for Error {}

pub struct EiEventConverter {
    pending_seats: HashMap<ei::Seat, SeatInner>,
    seats: HashMap<ei::Seat, Seat>,
    pending_devices: HashMap<ei::Device, DeviceInner>,
    devices: HashMap<ei::Device, Device>,
    device_for_interface: HashMap<Object, Device>,
    events: VecDeque<EiEvent>,
    pending_events: VecDeque<EiEvent>,
    serial: u32,
}

impl EiEventConverter {
    pub fn new(serial: u32) -> Self {
        Self {
            pending_seats: HashMap::new(),
            seats: HashMap::new(),
            pending_devices: HashMap::new(),
            devices: HashMap::new(),
            device_for_interface: HashMap::new(),
            events: VecDeque::new(),
            pending_events: VecDeque::new(),
            serial,
        }
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

    pub fn handle_event(&mut self, event: ei::Event) -> Result<(), Error> {
        match event {
            ei::Event::Handshake(_handshake, _event) => {
                return Err(Error::UnexpectedHandshakeEvent);
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
                    last_serial: _,
                    reason: _,
                    explanation: _,
                } => {
                    // TODO
                }
                ei::connection::Event::InvalidObject {
                    last_serial: _,
                    invalid_id: _,
                } => {
                    // TODO
                }
            },
            ei::Event::Callback(_callback, event) => match event {
                ei::callback::Event::Done { callback_data: _ } => {
                    // TODO
                }
            },
            ei::Event::Pingpong(_ping_pong, event) => match event {},
            ei::Event::Seat(seat, event) => match event {
                ei::seat::Event::Name { name } => {
                    let seat = self
                        .pending_seats
                        .get_mut(&seat)
                        .ok_or(Error::SeatSetupEventAfterDone)?;
                    seat.name = Some(name);
                }
                ei::seat::Event::Capability { mask, interface } => {
                    let seat = self
                        .pending_seats
                        .get_mut(&seat)
                        .ok_or(Error::SeatSetupEventAfterDone)?;
                    seat.capabilities.insert(interface, mask);
                }
                ei::seat::Event::Done => {
                    let seat = self
                        .pending_seats
                        .remove(&seat)
                        .ok_or(Error::SeatSetupEventAfterDone)?;
                    let seat = Seat(Arc::new(seat));
                    self.seats.insert(seat.0.seat.clone(), seat.clone());
                    self.queue_event(EiEvent::SeatAdded(SeatAdded { seat }));
                }
                ei::seat::Event::Device { device } => {
                    let seat = self
                        .seats
                        .get_mut(&seat)
                        .ok_or(Error::SeatEventBeforeDone)?;
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
                    self.serial = serial;
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
                        .ok_or(Error::DeviceSetupEventAfterDone)?;
                    device.name = Some(name);
                }
                ei::device::Event::DeviceType { device_type } => {
                    let device = self
                        .pending_devices
                        .get_mut(&device)
                        .ok_or(Error::DeviceSetupEventAfterDone)?;
                    device.device_type = Some(device_type);
                }
                ei::device::Event::Interface { object } => {
                    let device = self
                        .pending_devices
                        .get_mut(&device)
                        .ok_or(Error::DeviceSetupEventAfterDone)?;
                    device
                        .interfaces
                        .insert(object.interface().to_string(), object);
                }
                ei::device::Event::Dimensions { width, height } => {
                    let device = self
                        .pending_devices
                        .get_mut(&device)
                        .ok_or(Error::DeviceSetupEventAfterDone)?;
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
                        .ok_or(Error::DeviceSetupEventAfterDone)?;
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
                        .ok_or(Error::DeviceSetupEventAfterDone)?;
                    if device.device_type.is_none() {
                        return Err(Error::NoDeviceType);
                    }
                    let device = Device(Arc::new(device));
                    self.devices.insert(device.0.device.clone(), device.clone());
                    for i in device.0.interfaces.values() {
                        self.device_for_interface.insert(i.clone(), device.clone());
                    }
                    self.queue_event(EiEvent::DeviceAdded(DeviceAdded { device }));
                }
                ei::device::Event::Resumed { serial } => {
                    self.serial = serial;
                    let device = self
                        .devices
                        .get(&device)
                        .ok_or(Error::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::DeviceResumed(DeviceResumed {
                        device: device.clone(),
                        serial,
                    }));
                }
                ei::device::Event::Paused { serial } => {
                    self.serial = serial;
                    let device = self
                        .devices
                        .get(&device)
                        .ok_or(Error::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::DevicePaused(DevicePaused {
                        device: device.clone(),
                        serial,
                    }));
                }
                ei::device::Event::StartEmulating { serial, sequence } => {
                    self.serial = serial;
                    let device = self
                        .devices
                        .get(&device)
                        .ok_or(Error::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::DeviceStartEmulating(DeviceStartEmulating {
                        device: device.clone(),
                        serial,
                        sequence,
                    }));
                }
                ei::device::Event::StopEmulating { serial } => {
                    self.serial = serial;
                    let device = self
                        .devices
                        .get(&device)
                        .ok_or(Error::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::DeviceStopEmulating(DeviceStopEmulating {
                        device: device.clone(),
                        serial,
                    }));
                }
                ei::device::Event::RegionMappingId { mapping_id } => {
                    let device = self
                        .pending_devices
                        .get_mut(&device)
                        .ok_or(Error::DeviceEventBeforeDone)?;
                    device.next_region_mapping_id = Some(mapping_id);
                }
                ei::device::Event::Frame { serial, timestamp } => {
                    self.serial = serial;
                    let device = self
                        .devices
                        .get(&device)
                        .ok_or(Error::DeviceEventBeforeDone)?;
                    self.queue_event(EiEvent::Frame(Frame {
                        device: device.clone(),
                        serial,
                        time: timestamp,
                    }));
                }
                ei::device::Event::Destroyed { serial } => {
                    self.serial = serial;
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
                        .ok_or(Error::DeviceSetupEventAfterDone)?;
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
                        .ok_or(Error::DeviceEventBeforeDone)?;
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
                    self.serial = serial;
                    let device = self
                        .device_for_interface
                        .get(&keyboard.0)
                        .ok_or(Error::DeviceEventBeforeDone)?;
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
                    self.serial = serial;
                    // TODO does interface need to be removed from `Device`?
                    self.device_for_interface.remove(&keyboard.0);
                }
            },
            ei::Event::Pointer(pointer, event) => {
                let device = self
                    .device_for_interface
                    .get(&pointer.0)
                    .ok_or(Error::DeviceEventBeforeDone)?;
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
                        self.serial = serial;
                        // TODO does interface need to be removed from `Device`?
                        self.device_for_interface.remove(&pointer.0);
                    }
                }
            }
            ei::Event::PointerAbsolute(pointer_absolute, event) => {
                let device = self
                    .device_for_interface
                    .get(&pointer_absolute.0)
                    .ok_or(Error::DeviceEventBeforeDone)?;
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
                        self.serial = serial;
                        // TODO does interface need to be removed from `Device`?
                        self.device_for_interface.remove(&pointer_absolute.0);
                    }
                }
            }
            ei::Event::Scroll(scroll, event) => {
                let device = self
                    .device_for_interface
                    .get(&scroll.0)
                    .ok_or(Error::DeviceEventBeforeDone)?;
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
                        self.serial = serial;
                        // TODO does interface need to be removed from `Device`?
                        self.device_for_interface.remove(&scroll.0);
                    }
                }
            }
            ei::Event::Button(button, event) => {
                let device = self
                    .device_for_interface
                    .get(&button.0)
                    .ok_or(Error::DeviceEventBeforeDone)?;
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
                        self.serial = serial;
                        // TODO does interface need to be removed from `Device`?
                        self.device_for_interface.remove(&button.0);
                    }
                }
            }
            ei::Event::Touchscreen(touchscreen, event) => {
                let device = self
                    .device_for_interface
                    .get(&touchscreen.0)
                    .ok_or(Error::DeviceEventBeforeDone)?;
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
                    ei::touchscreen::Event::Destroyed { serial } => {
                        self.serial = serial;
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

    pub fn serial(&self) -> u32 {
        self.serial
    }
}

#[derive(Debug)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub mapping_id: Option<String>,
}

#[derive(Debug)]
pub struct Keymap {
    pub fd: OwnedFd,
    pub size: u32,
    pub type_: ei::keyboard::KeymapType,
}

// bitflags?
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

#[derive(Clone)]
pub struct Seat(Arc<SeatInner>);

impl fmt::Debug for Seat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = self.name() {
            write!(f, "Seat(\"{}\")", name)
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

#[derive(Clone)]
pub struct Device(Arc<DeviceInner>);

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = self.name() {
            write!(f, "Device(\"{}\")", name)
        } else {
            write!(f, "Device(None)")
        }
    }
}

impl Device {
    pub fn seat(&self) -> &Seat {
        &self.0.seat
    }

    pub fn device(&self) -> &ei::Device {
        &self.0.device
    }

    pub fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    pub fn device_type(&self) -> ei::device::DeviceType {
        self.0.device_type.unwrap()
    }

    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.0.dimensions
    }

    pub fn regions(&self) -> &[Region] {
        &self.0.regions
    }

    pub fn keymap(&self) -> Option<&Keymap> {
        self.0.keymap.as_ref()
    }

    pub fn interface<T: ei::Interface>(&self) -> Option<T> {
        self.0.interfaces.get(T::NAME)?.clone().downcast()
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

#[derive(Clone, Debug, PartialEq)]
pub enum EiEvent {
    // Connect,
    // Disconnect,
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
}

impl EiEvent {
    // Events that are grouped by frames need their times set when the
    // frame event occurs.
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
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeatAdded {
    pub seat: Seat,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeatRemoved {
    pub seat: Seat,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceAdded {
    pub device: Device,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceRemoved {
    pub device: Device,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DevicePaused {
    pub device: Device,
    pub serial: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceResumed {
    pub device: Device,
    pub serial: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyboardModifiers {
    pub device: Device,
    pub serial: u32,
    pub depressed: u32,
    pub latched: u32,
    pub locked: u32,
    pub group: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub device: Device,
    pub serial: u32,
    pub time: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStartEmulating {
    pub device: Device,
    pub serial: u32,
    pub sequence: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStopEmulating {
    pub device: Device,
    pub serial: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointerMotion {
    pub device: Device,
    pub time: u64,
    pub dx: f32,
    pub dy: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointerMotionAbsolute {
    pub device: Device,
    pub time: u64,
    pub dx_absolute: f32,
    pub dy_absolute: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    pub device: Device,
    pub time: u64,
    pub button: u32,
    pub state: ei::button::ButtonState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDelta {
    pub device: Device,
    pub time: u64,
    pub dx: f32,
    pub dy: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollStop {
    pub device: Device,
    pub time: u64,
    pub x: bool,
    pub y: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollCancel {
    pub device: Device,
    pub time: u64,
    pub x: bool,
    pub y: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDiscrete {
    pub device: Device,
    pub time: u64,
    pub discrete_dx: i32,
    pub discrete_dy: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyboardKey {
    pub device: Device,
    pub time: u64,
    pub key: u32,
    pub state: ei::keyboard::KeyState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TouchDown {
    pub device: Device,
    pub time: u64,
    pub touch_id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TouchMotion {
    pub device: Device,
    pub time: u64,
    pub touch_id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TouchUp {
    pub device: Device,
    pub time: u64,
    pub touch_id: u32,
}

pub trait SeatEvent {
    fn seat(&self) -> &Seat;
}

pub trait DeviceEvent: SeatEvent {
    fn device(&self) -> &Device;
}

pub trait EventTime: DeviceEvent {
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

#[derive(Debug)]
pub enum EiConvertEventIteratorError {
    Io(io::Error),
    Parse(ParseError),
    Event(crate::event::Error),
}

pub struct EiConvertEventIterator {
    context: ei::Context,
    converter: EiEventConverter,
}

impl EiConvertEventIterator {
    pub fn new(context: ei::Context, serial: u32) -> Self {
        Self {
            context,
            converter: EiEventConverter::new(serial),
        }
    }
}

impl Iterator for EiConvertEventIterator {
    type Item = Result<crate::event::EiEvent, EiConvertEventIteratorError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(event) = self.converter.next_event() {
                return Some(Ok(event));
            }
            if let Err(err) = util::poll_readable(&self.context) {
                return Some(Err(EiConvertEventIteratorError::Io(err)));
            }
            match self.context.read() {
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return None,
                Err(err) => return Some(Err(EiConvertEventIteratorError::Io(err))),
                Ok(_) => {}
            };
            while let Some(result) = self.context.pending_event() {
                let request = match result {
                    PendingRequestResult::Request(request) => request,
                    PendingRequestResult::ParseError(parse_error) => {
                        return Some(Err(EiConvertEventIteratorError::Parse(parse_error)))
                    }
                    PendingRequestResult::InvalidObject(_invalid_object) => {
                        // Log?
                        continue;
                    }
                };

                if let Err(err) = self.converter.handle_event(request) {
                    return Some(Err(EiConvertEventIteratorError::Event(err)));
                }
            }
        }
    }
}
