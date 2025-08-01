//! High-level server-side wrappers for common objects and their requests.

#![allow(clippy::derive_partial_eq_without_eq)]

// TODO: rename/reorganize things; doc comments on public types/methods

use crate::{
    ei::connection::DisconnectReason, eis, handshake::EisHandshakeResp, wire::Interface, Error,
    Object,
};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    sync::{Arc, Mutex, Weak},
};

pub use crate::event::DeviceCapability;

#[derive(Debug)]
struct ConnectionInner {
    context: eis::Context,
    handshake_resp: EisHandshakeResp,
    seats: Mutex<HashMap<eis::Seat, Seat>>,
    devices: Mutex<HashMap<eis::Device, Device>>,
    device_for_interface: Mutex<HashMap<Object, Device>>,
    last_serial: Mutex<u32>,
}

/// High-level server-side wrapper for `ei_connection`.
#[derive(Clone, Debug)]
pub struct Connection(Arc<ConnectionInner>);

impl Connection {
    /// Returns the interface proxy for the underlying `ei_connection` object.
    pub fn connection(&self) -> &eis::Connection {
        &self.0.handshake_resp.connection
    }

    /// Notifies the client that the connection will close.
    ///
    /// When a client is disconnected due to an error, `reason` must be something other than
    /// [`DisconnectReason::Disconnected`], and `explanation` may contain a string explaining
    /// why.
    ///
    /// When a client is disconnected on purpose, for example after a user interaction,
    /// `reason` must be [`DisconnectReason::Disconnected`], and `explanation` must be `None`.
    // TODO(axka, 2025-07-08): rename to something imperative like `notify_disconnection`
    // TODO(axka, 2025-07-08): `explanation` must support NULL: https://gitlab.freedesktop.org/libinput/libei/-/commit/267716a7609730914b24adf5860ec8d2cf2e7636
    pub fn disconnected(&self, reason: DisconnectReason, explanation: &str) {
        let seats = self
            .0
            .seats
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for seat in seats {
            seat.remove();
        }
        self.connection()
            .disconnected(self.last_serial(), reason, explanation);
    }

    /// Sends buffered messages. Call after you're finished with sending events.
    pub fn flush(&self) -> rustix::io::Result<()> {
        self.0.context.flush()
    }

    /// Returns the context type of this this connection.
    ///
    /// That is — whether the client emulates input events via requests or receives
    /// input events.
    pub fn context_type(&self) -> eis::handshake::ContextType {
        self.0.handshake_resp.context_type
    }

    /// Returns the human-readable name of the client.
    pub fn name(&self) -> Option<&str> {
        self.0.handshake_resp.name.as_deref()
    }

    // Use type instead of string?
    /// Returns `true` if the connection supports the named interface.
    pub fn has_interface(&self, interface: &str) -> bool {
        self.0
            .handshake_resp
            .negotiated_interfaces
            .contains_key(interface)
    }

    /// Returns the version of the named interface if it's supported on this
    /// connection. Otherwise returns `None`.
    pub fn interface_version(&self, interface: &str) -> Option<u32> {
        self.0
            .handshake_resp
            .negotiated_interfaces
            .get(interface)
            .copied()
    }

    // TODO(axka, 2025-07-08): specify in the function name that this is the last serial from
    // the server, and not the client, and create a function for the other way around.
    /// Returns the last serial used in an event sent by the server.
    pub fn last_serial(&self) -> u32 {
        *self.0.last_serial.lock().unwrap()
    }

    /// Increments the current serial and runs the provided callback with it.
    pub fn with_next_serial<T, F: FnOnce(u32) -> T>(&self, cb: F) -> T {
        let mut last_serial = self.0.last_serial.lock().unwrap();
        let serial = last_serial.wrapping_add(1);
        let res = cb(serial);
        *last_serial = serial;
        res
    }

    fn device_for_interface<T: DeviceInterface>(&mut self, interface: &T) -> Option<Device> {
        self.0
            .device_for_interface
            .lock()
            .unwrap()
            .get(interface.as_object())
            .cloned()
    }

    /// Adds a seat to the connection.
    pub fn add_seat(&self, name: Option<&str>, capabilities: &[DeviceCapability]) -> Seat {
        let seat = self.connection().seat(1);
        if let Some(name) = name {
            seat.name(name);
        }
        for capability in capabilities {
            // TODO only send negotiated interfaces
            seat.capability(2 << *capability as u64, capability.name());
        }
        seat.done();
        let seat = Seat(Arc::new(SeatInner {
            seat,
            name: name.map(|x| x.to_owned()),
            handle: Arc::downgrade(&self.0),
        }));
        self.0
            .seats
            .lock()
            .unwrap()
            .insert(seat.0.seat.clone(), seat.clone());
        seat
    }
}

// TODO libei has a `eis_clock_set_now_func`
// Return time in us
fn eis_now() -> u64 {
    let time = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    time.tv_sec as u64 * 1_000_000 + time.tv_nsec as u64 / 1_000
}

// need way to add seat/device?
/// Utility that converts low-level protocol-level requests into high-level requests defined in
/// this module.
#[derive(Debug)]
pub struct EisRequestConverter {
    requests: VecDeque<EisRequest>,
    pending_requests: VecDeque<EisRequest>,
    handle: Connection,
}

impl EisRequestConverter {
    /// Creates a new converter.
    pub fn new(
        context: &eis::Context,
        handshake_resp: EisHandshakeResp,
        initial_serial: u32,
    ) -> Self {
        Self {
            requests: VecDeque::new(),
            pending_requests: VecDeque::new(),
            handle: Connection(Arc::new(ConnectionInner {
                context: context.clone(),
                handshake_resp,
                seats: Default::default(),
                devices: Default::default(),
                device_for_interface: Default::default(),
                last_serial: Mutex::new(initial_serial),
            })),
        }
    }

    pub fn handle(&self) -> &Connection {
        &self.handle
    }

    fn queue_frame_event(&mut self, device: &Device) {
        self.queue_request(EisRequest::Frame(Frame {
            time: eis_now(),
            device: device.clone(),
            last_serial: self.handle.last_serial(),
        }))
    }

    // Based on behavior of `eis_queue_request` in libeis
    fn queue_request(&mut self, mut request: EisRequest) {
        if request.time_mut().is_some() {
            self.pending_requests.push_back(request);
        } else if let EisRequest::Frame(Frame { time, .. }) = &request {
            if self.pending_requests.is_empty() {
                return;
            }
            for mut pending_request in self.pending_requests.drain(..) {
                *pending_request.time_mut().unwrap() = *time;
                self.requests.push_back(pending_request);
            }
            self.requests.push_back(request);
        } else {
            if let Some(device) = request.device() {
                if !self.pending_requests.is_empty() {
                    self.queue_frame_event(device);
                }
            }
            self.requests.push_back(request);
        }
    }

    pub fn next_request(&mut self) -> Option<EisRequest> {
        self.requests.pop_front()
    }

    pub fn handle_request(&mut self, request: eis::Request) -> Result<(), Error> {
        match request {
            eis::Request::Handshake(_handshake, _request) => {
                return Err(Error::UnexpectedHandshakeEvent);
            }
            eis::Request::Connection(_connection, request) => match request {
                eis::connection::Request::Sync { callback } => {
                    if callback.version() != 1 {
                        return Err(Error::InvalidInterfaceVersion(
                            "ei_callback",
                            callback.version(),
                        ));
                    }
                    callback.done(0);
                    if let Some(backend) = callback.0.backend() {
                        // XXX Error?
                        let _ = backend.flush();
                    }
                }
                eis::connection::Request::Disconnect => {
                    self.queue_request(EisRequest::Disconnect);
                }
            },
            eis::Request::Callback(_callback, request) => match request {},
            eis::Request::Pingpong(_ping_pong, request) => match request {
                eis::pingpong::Request::Done { callback_data: _ } => {
                    // TODO
                }
            },
            eis::Request::Seat(seat, request) => match request {
                eis::seat::Request::Release => {
                    self.handle
                        .with_next_serial(|serial| seat.destroyed(serial));
                }
                eis::seat::Request::Bind { capabilities } => {
                    let Some(seat) = self.handle.0.seats.lock().unwrap().get(&seat).cloned() else {
                        return Ok(());
                    };
                    self.queue_request(EisRequest::Bind(Bind { seat, capabilities }));
                }
            },
            eis::Request::Device(device, request) => {
                let Some(device) = self.handle.0.devices.lock().unwrap().get(&device).cloned()
                else {
                    return Ok(());
                };
                match request {
                    eis::device::Request::Release => {}
                    eis::device::Request::StartEmulating {
                        last_serial,
                        sequence,
                    } => {
                        self.queue_request(EisRequest::DeviceStartEmulating(
                            DeviceStartEmulating {
                                device,
                                last_serial,
                                sequence,
                            },
                        ));
                    }
                    eis::device::Request::StopEmulating { last_serial } => {
                        self.queue_request(EisRequest::DeviceStopEmulating(DeviceStopEmulating {
                            device,
                            last_serial,
                        }));
                    }
                    eis::device::Request::Frame {
                        last_serial,
                        timestamp,
                    } => {
                        self.queue_request(EisRequest::Frame(Frame {
                            device,
                            last_serial,
                            time: timestamp,
                        }));
                    }
                }
            }
            eis::Request::Keyboard(keyboard, request) => {
                let Some(device) = self.handle.device_for_interface(&keyboard) else {
                    return Ok(());
                };
                match request {
                    eis::keyboard::Request::Release => {}
                    eis::keyboard::Request::Key { key, state } => {
                        self.queue_request(EisRequest::KeyboardKey(KeyboardKey {
                            device,
                            key,
                            state,
                            time: 0,
                        }));
                    }
                }
            }
            eis::Request::Pointer(pointer, request) => {
                let Some(device) = self.handle.device_for_interface(&pointer) else {
                    return Ok(());
                };
                match request {
                    eis::pointer::Request::Release => {}
                    eis::pointer::Request::MotionRelative { x, y } => {
                        self.queue_request(EisRequest::PointerMotion(PointerMotion {
                            device,
                            dx: x,
                            dy: y,
                            time: 0,
                        }));
                    }
                }
            }
            eis::Request::PointerAbsolute(pointer_absolute, request) => {
                let Some(device) = self.handle.device_for_interface(&pointer_absolute) else {
                    return Ok(());
                };
                match request {
                    eis::pointer_absolute::Request::Release => {}
                    eis::pointer_absolute::Request::MotionAbsolute { x, y } => {
                        self.queue_request(EisRequest::PointerMotionAbsolute(
                            PointerMotionAbsolute {
                                device,
                                dx_absolute: x,
                                dy_absolute: y,
                                time: 0,
                            },
                        ));
                    }
                }
            }
            eis::Request::Scroll(scroll, request) => {
                let Some(device) = self.handle.device_for_interface(&scroll) else {
                    return Ok(());
                };
                match request {
                    eis::scroll::Request::Release => {}
                    eis::scroll::Request::Scroll { x, y } => {
                        self.queue_request(EisRequest::ScrollDelta(ScrollDelta {
                            device,
                            dx: x,
                            dy: y,
                            time: 0,
                        }));
                    }
                    eis::scroll::Request::ScrollDiscrete { x, y } => {
                        self.queue_request(EisRequest::ScrollDiscrete(ScrollDiscrete {
                            device,
                            discrete_dx: x,
                            discrete_dy: y,
                            time: 0,
                        }));
                    }
                    eis::scroll::Request::ScrollStop { x, y, is_cancel } => {
                        if is_cancel != 0 {
                            self.queue_request(EisRequest::ScrollCancel(ScrollCancel {
                                device,
                                time: 0,
                                x: x != 0,
                                y: y != 0,
                            }));
                        } else {
                            self.queue_request(EisRequest::ScrollStop(ScrollStop {
                                device,
                                time: 0,
                                x: x != 0,
                                y: y != 0,
                            }));
                        }
                    }
                }
            }
            eis::Request::Button(button, request) => {
                let Some(device) = self.handle.device_for_interface(&button) else {
                    return Ok(());
                };
                match request {
                    eis::button::Request::Release => {}
                    eis::button::Request::Button { button, state } => {
                        self.queue_request(EisRequest::Button(Button {
                            device,
                            button,
                            state,
                            time: 0,
                        }));
                    }
                }
            }
            eis::Request::Touchscreen(touchscreen, request) => {
                let Some(device) = self.handle.device_for_interface(&touchscreen) else {
                    return Ok(());
                };
                match request {
                    eis::touchscreen::Request::Release => {}
                    eis::touchscreen::Request::Down { touchid, x, y } => {
                        self.queue_request(EisRequest::TouchDown(TouchDown {
                            device,
                            touch_id: touchid,
                            x,
                            y,
                            time: 0,
                        }));
                    }
                    eis::touchscreen::Request::Motion { touchid, x, y } => {
                        self.queue_request(EisRequest::TouchMotion(TouchMotion {
                            device,
                            touch_id: touchid,
                            x,
                            y,
                            time: 0,
                        }));
                    }
                    eis::touchscreen::Request::Up { touchid } => {
                        self.queue_request(EisRequest::TouchUp(TouchUp {
                            device,
                            touch_id: touchid,
                            time: 0,
                        }));
                    }
                    eis::touchscreen::Request::Cancel { touchid } => {
                        if touchscreen.version() < 2 {
                            return Err(Error::InvalidInterfaceVersion(
                                "ei_touchscreen",
                                touchscreen.version(),
                            ));
                        }
                        self.queue_request(EisRequest::TouchCancel(TouchCancel {
                            device,
                            touch_id: touchid,
                            time: 0,
                        }));
                    }
                }
            }
        }
        Ok(())
    }
}

struct SeatInner {
    seat: eis::Seat,
    name: Option<String>,
    //capabilities: HashMap<String, u64>,
    handle: Weak<ConnectionInner>,
}

/// High-level server-side wrapper for `ei_seat`.
#[derive(Clone)]
pub struct Seat(Arc<SeatInner>);

fn add_interface<I: eis::Interface>(
    device: &eis::Device,
    connection: Option<&Connection>,
) -> Object {
    // TODO better way to handle dead connection?
    let version = connection
        .as_ref()
        .and_then(|c| c.interface_version(I::NAME))
        .unwrap_or(1);
    device.interface::<I>(version).as_object().clone()
}

impl Seat {
    /// Returns the interface proxy for the underlying `ei_seat` object.
    pub fn eis_seat(&self) -> &eis::Seat {
        &self.0.seat
    }

    // builder pattern?
    /// Adds a device to the connection.
    pub fn add_device(
        &self,
        name: Option<&str>,
        device_type: eis::device::DeviceType,
        capabilities: &[DeviceCapability],
        // TODO: better solution; keymap, etc.
        before_done_cb: impl for<'a> FnOnce(&'a Device),
    ) -> Device {
        let connection = self.0.handle.upgrade().map(Connection);

        let device = self.0.seat.device(1);
        if let Some(name) = name {
            device.name(name);
        }
        device.device_type(device_type);
        // TODO
        // dimensions
        // regions; region_mapping_id
        // TODO add interfaces for capabilities
        // - `eis_device_configure_capability`; only if seat has capability
        let mut interfaces = HashMap::new();
        for capability in capabilities {
            let object = match capability {
                DeviceCapability::Pointer => {
                    add_interface::<eis::Pointer>(&device, connection.as_ref())
                }
                DeviceCapability::PointerAbsolute => {
                    add_interface::<eis::PointerAbsolute>(&device, connection.as_ref())
                }
                DeviceCapability::Keyboard => {
                    add_interface::<eis::Keyboard>(&device, connection.as_ref())
                }
                DeviceCapability::Touch => {
                    add_interface::<eis::Touchscreen>(&device, connection.as_ref())
                }
                DeviceCapability::Scroll => {
                    add_interface::<eis::Scroll>(&device, connection.as_ref())
                }
                DeviceCapability::Button => {
                    add_interface::<eis::Button>(&device, connection.as_ref())
                }
            };
            interfaces.insert(object.interface().to_string(), object);
        }

        let device = Device(Arc::new(DeviceInner {
            device,
            seat: self.clone(),
            name: name.map(|x| x.to_string()),
            interfaces,
            handle: self.0.handle.clone(),
        }));
        if let Some(handle) = connection {
            for interface in device.0.interfaces.values() {
                handle
                    .0
                    .device_for_interface
                    .lock()
                    .unwrap()
                    .insert(interface.clone(), device.clone());
            }
            handle
                .0
                .devices
                .lock()
                .unwrap()
                .insert(device.0.device.clone(), device.clone());
        }

        before_done_cb(&device);
        device.device().done();

        device
    }

    /// Removes this seat and associated devices from the connection.
    pub fn remove(&self) {
        if let Some(handle) = self.0.handle.upgrade().map(Connection) {
            let devices = handle
                .0
                .devices
                .lock()
                .unwrap()
                .values()
                .filter(|device| &device.0.seat == self)
                .cloned()
                .collect::<Vec<_>>();
            for device in devices {
                device.remove();
            }

            handle.with_next_serial(|serial| self.0.seat.destroyed(serial));
            handle.0.seats.lock().unwrap().remove(&self.0.seat);
        }
    }
}

impl fmt::Debug for Seat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = &self.0.name {
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

pub trait DeviceInterface: eis::Interface {}

macro_rules! impl_device_interface {
    ($ty:ty) => {
        impl DeviceInterface for $ty {}
    };
}
impl_device_interface!(eis::Pointer);
impl_device_interface!(eis::PointerAbsolute);
impl_device_interface!(eis::Scroll);
impl_device_interface!(eis::Button);
impl_device_interface!(eis::Keyboard);
impl_device_interface!(eis::Touchscreen);

fn destroy_interface(object: crate::Object, serial: u32) {
    match object.interface() {
        eis::Pointer::NAME => object
            .downcast_unchecked::<eis::Pointer>()
            .destroyed(serial),
        eis::PointerAbsolute::NAME => object
            .downcast_unchecked::<eis::PointerAbsolute>()
            .destroyed(serial),
        eis::Scroll::NAME => object.downcast_unchecked::<eis::Scroll>().destroyed(serial),
        eis::Button::NAME => object.downcast_unchecked::<eis::Button>().destroyed(serial),
        eis::Keyboard::NAME => object
            .downcast_unchecked::<eis::Keyboard>()
            .destroyed(serial),
        eis::Touchscreen::NAME => object
            .downcast_unchecked::<eis::Touchscreen>()
            .destroyed(serial),
        _ => unreachable!(),
    }
}

struct DeviceInner {
    device: eis::Device,
    seat: Seat,
    name: Option<String>,
    interfaces: HashMap<String, crate::Object>,
    handle: Weak<ConnectionInner>,
}

/// High-level server-side wrapper for `ei_device`.
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
    /// Returns the high-level [`Seat`] wrapper for this device.
    pub fn seat(&self) -> &Seat {
        &self.0.seat
    }

    /// Returns the interface proxy for the underlying `ei_device` object.
    pub fn device(&self) -> &eis::Device {
        &self.0.device
    }

    /// Returns the name of the device.
    pub fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    /// Returns an interface proxy if it is implemented for this device.
    ///
    /// Interfaces of devices are implemented, such that there is one `ei_device` object and other objects (for example `ei_keyboard`) denoting capabilities.
    pub fn interface<T: DeviceInterface>(&self) -> Option<T> {
        self.0.interfaces.get(T::NAME)?.clone().downcast()
    }

    /// Returns `true` if this device has an interface matching the provided capability.
    pub fn has_capability(&self, capability: DeviceCapability) -> bool {
        self.0.interfaces.contains_key(capability.name())
    }

    /// Removes this device and associated interfaces from the connection.
    pub fn remove(&self) {
        if let Some(handle) = self.0.handle.upgrade().map(Connection) {
            for interface in self.0.interfaces.values() {
                handle
                    .0
                    .device_for_interface
                    .lock()
                    .unwrap()
                    .remove(interface);
                handle.with_next_serial(|serial| destroy_interface(interface.clone(), serial));
            }

            handle.with_next_serial(|serial| self.0.device.destroyed(serial));
            handle.0.devices.lock().unwrap().remove(&self.0.device);
        }
    }

    /// Notifies to the client that, depending on the context type, it may request to start emulating or receiving input events. A newly advertised device is in the [`paused`](Self::paused) state.
    ///
    /// See [`eis::Device::resumed`] for documentation from the protocol specification.
    pub fn resumed(&self) {
        if let Some(handle) = self.0.handle.upgrade().map(Connection) {
            handle.with_next_serial(|serial| self.device().resumed(serial))
        }
    }

    /// Notifies to the client that, depending on the context type, no further input events
    /// will be accepted for emulation or no further input events will be sent.
    ///
    /// See [`eis::Device::paused`] for documentation from the protocol specification.
    pub fn paused(&self) {
        if let Some(handle) = self.0.handle.upgrade().map(Connection) {
            handle.with_next_serial(|serial| self.device().paused(serial))
        }
    }

    // TODO: statically restrict the below to receiver context?

    /// Notifies the client that the given device is about to start sending events.
    ///
    /// **Note:** Must only be sent in a receiver context.
    ///
    /// See [`eis::Device::start_emulating`] for documentation from the protocol specification.
    pub fn start_emulating(&self, sequence: u32) {
        if let Some(handle) = self.0.handle.upgrade().map(Connection) {
            handle.with_next_serial(|serial| self.device().start_emulating(serial, sequence))
        }
    }

    /// Notifies the client that the given device is no longer sending events.
    ///
    /// **Note:** Must only be sent in a receiver context.
    ///
    /// See [`eis::Device::stop_emulating`] for documentation from the protocol specification.
    pub fn stop_emulating(&self) {
        if let Some(handle) = self.0.handle.upgrade().map(Connection) {
            handle.with_next_serial(|serial| self.device().stop_emulating(serial))
        }
    }

    /// Notifies the client to group the current set of events into a logical hardware
    /// event.
    ///
    /// **Note:** Must only be sent in a receiver context.
    ///
    /// See [`eis::Device::frame`] for documentation from the protocol specification.
    pub fn frame(&self, time: u64) {
        if let Some(handle) = self.0.handle.upgrade().map(Connection) {
            handle.with_next_serial(|serial| self.device().frame(serial, time))
        }
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

/// Enum containing all possible requests the high-level utilities will give for a server implementation to handle.
#[derive(Clone, Debug, PartialEq)]
pub enum EisRequest {
    // TODO connect, disconnect, device closed
    Disconnect,
    Bind(Bind),
    // Only for sender context
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

impl EisRequest {
    // Requests that are grouped by frames need their times set when the
    // frame request occurs.
    /// Returns the `time` property of this request, if applicable.
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
            Self::Disconnect
            | Self::Bind(_)
            | Self::Frame(_)
            | Self::DeviceStartEmulating(_)
            | Self::DeviceStopEmulating(_) => None,
        }
    }

    /// Returns the high-level [`Device`] wrapper for this request, if applicable.
    pub fn device(&self) -> Option<&Device> {
        match self {
            Self::Frame(evt) => Some(&evt.device),
            Self::DeviceStartEmulating(evt) => Some(&evt.device),
            Self::DeviceStopEmulating(evt) => Some(&evt.device),
            Self::PointerMotion(evt) => Some(&evt.device),
            Self::PointerMotionAbsolute(evt) => Some(&evt.device),
            Self::Button(evt) => Some(&evt.device),
            Self::ScrollDelta(evt) => Some(&evt.device),
            Self::ScrollStop(evt) => Some(&evt.device),
            Self::ScrollCancel(evt) => Some(&evt.device),
            Self::ScrollDiscrete(evt) => Some(&evt.device),
            Self::KeyboardKey(evt) => Some(&evt.device),
            Self::TouchDown(evt) => Some(&evt.device),
            Self::TouchUp(evt) => Some(&evt.device),
            Self::TouchMotion(evt) => Some(&evt.device),
            Self::TouchCancel(evt) => Some(&evt.device),
            Self::Disconnect | Self::Bind(_) => None,
        }
    }
}

/// High-level translation of [`ei_seat.bind`](eis::seat::Request::Bind).
#[derive(Clone, Debug, PartialEq)]
pub struct Bind {
    /// High-level [`Seat`] wrapper.
    pub seat: Seat,
    pub capabilities: u64,
}

/// High-level translation of [`ei_device.frame`](eis::device::Request::Frame).
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    pub last_serial: u32,
    /// Timestamp in microseconds.
    pub time: u64,
}

/// High-level translation of [`ei_device.start_emulating`](eis::device::Request::StartEmulating).
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStartEmulating {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    pub last_serial: u32,
    pub sequence: u32,
}

/// High-level translation of [`ei_device.stop_emulating`](eis::device::Request::StopEmulating).
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStopEmulating {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    pub last_serial: u32,
}

/// High-level translation of [`ei_pointer.motion_relative`](eis::pointer::Request::MotionRelative).
#[derive(Clone, Debug, PartialEq)]
pub struct PointerMotion {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub dx: f32,
    pub dy: f32,
}

/// High-level translation of [`ei_pointer_absolute.motion_absolute`](eis::pointer_absolute::Request::MotionAbsolute).
#[derive(Clone, Debug, PartialEq)]
pub struct PointerMotionAbsolute {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub dx_absolute: f32,
    pub dy_absolute: f32,
}

/// High-level translation of [`ei_button.button`](eis::button::Request::Button).
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub button: u32,
    pub state: eis::button::ButtonState,
}

/// High-level translation of [`ei_scroll.scroll`](eis::scroll::Request::Scroll).
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDelta {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub dx: f32,
    pub dy: f32,
}

/// High-level translation of [`ei_scroll.scroll_stop`](eis::scroll::Request::ScrollStop) when its `is_cancel` is zero.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollStop {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub x: bool,
    pub y: bool,
}

/// High-level translation of [`ei_scroll.scroll_stop`](eis::scroll::Request::ScrollStop) when its `is_cancel` is nonzero.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollCancel {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub x: bool,
    pub y: bool,
}

/// High-level translation of [`ei_scroll.scroll_discrete`](eis::scroll::Request::ScrollDiscrete).
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDiscrete {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    pub discrete_dx: i32,
    pub discrete_dy: i32,
}

/// High-level translation of [`ei_keyboard.key`](eis::keyboard::Request::Key).
#[derive(Clone, Debug, PartialEq)]
pub struct KeyboardKey {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Key code (according to the current keymap, if any).
    pub key: u32,
    /// Logical key state.
    pub state: eis::keyboard::KeyState,
}

/// High-level translation of [`ei_touchscreen.down`](eis::touchscreen::Request::Down).
#[derive(Clone, Debug, PartialEq)]
pub struct TouchDown {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Unique touch ID, defined in this request.
    pub touch_id: u32,
    pub x: f32,
    pub y: f32,
}

/// High-level translation of [`ei_touchscreen.motion`](eis::touchscreen::Request::Motion).
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

/// High-level translation of [`ei_touchscreen.up`](eis::touchscreen::Request::Up).
#[derive(Clone, Debug, PartialEq)]
pub struct TouchUp {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Unique touch ID, defined in [`TouchDown`]. It may be reused after this request.
    pub touch_id: u32,
}

/// High-level translation of [`ei_touchscreen.chcancel`](eis::touchscreen::Request::Cancel).
#[derive(Clone, Debug, PartialEq)]
pub struct TouchCancel {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Unique touch ID, defined in [`TouchDown`].
    pub touch_id: u32,
}

// TODO(axka, 2025-07-08): event and request terms collide when the below traits are implemented on
// variants of `EisRequest`. Furthermore, the name of the module is slightly confusing.

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
