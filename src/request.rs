//! High-level server-side wrappers for common objects and their requests.

#![allow(clippy::derive_partial_eq_without_eq)]

// TODO: rename/reorganize things; doc comments on public types/methods

use enumflags2::{BitFlag, BitFlags};

use crate::{
    ei::connection::DisconnectReason, eis, handshake::EisHandshakeResp, wire::Interface, Error,
    Object,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
};

pub use crate::event::DeviceCapability;

// For compatability, defined the same way as libei
const EIS_MAX_TOUCHES: usize = 16;

/// Protocol errors of the client.
#[derive(Debug)]
pub enum RequestError {
    /// Invalid capabilities in `ei_seat.bind`.
    InvalidCapabilities,
    /// Touch down even duplicated
    DuplicatedTouchDown,
    /// Too many touches
    TooManyTouches,
}
impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidCapabilities => write!(f, "Invalid capabilities"),
            Self::DuplicatedTouchDown => write!(f, "Touch down event for duplicated touch ID"),
            Self::TooManyTouches => write!(f, "Too many simultaneous touch events"),
        }
    }
}

#[derive(Debug)]
struct ConnectionInner {
    context: eis::Context,
    handshake_resp: EisHandshakeResp,
    seats: Mutex<HashMap<eis::Seat, Seat>>,
    devices: Mutex<HashMap<eis::Device, Device>>,
    device_for_interface: Mutex<HashMap<Object, Device>>,
    last_serial: Mutex<u32>,
    disconnected: AtomicBool,
}

/// High-level server-side wrapper for `ei_connection`.
#[derive(Clone, Debug)]
pub struct Connection(Arc<ConnectionInner>);

impl Connection {
    /// Returns the interface proxy for the underlying `ei_connection` object.
    #[must_use]
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
    ///
    /// # Panics
    ///
    /// Will panic if an internal Mutex is poisoned.
    // TODO(axka, 2025-07-08): rename to something imperative like `notify_disconnection`
    pub fn disconnected(&self, reason: DisconnectReason, explanation: Option<&str>) {
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
        // If flush fails because buffer is full, client can just get an EOF without
        // a message.
        let _ = self.flush();
        self.0.disconnected.store(true, Ordering::SeqCst);
        // Shutdown read end of socket, so anything reading/polling it will get EOF,
        // without waiting for client to disconnect first.
        self.0.context.0.shutdown_read();
    }

    #[cfg(feature = "calloop")]
    pub(crate) fn has_sent_disconnected(&self) -> bool {
        self.0.disconnected.load(Ordering::SeqCst)
    }

    /// Sends buffered messages. Call after you're finished with sending events.
    ///
    /// # Errors
    ///
    /// An error will be returned if sending the buffered messages fails.
    pub fn flush(&self) -> rustix::io::Result<()> {
        self.0.context.flush()
    }

    /// Returns the context type of this this connection.
    ///
    /// That is — whether the client emulates input events via requests or receives
    /// input events.
    #[must_use]
    pub fn context_type(&self) -> eis::handshake::ContextType {
        self.0.handshake_resp.context_type
    }

    /// Returns the human-readable name of the client.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.0.handshake_resp.name.as_deref()
    }

    // Use type instead of string?
    /// Returns `true` if the connection has negotiated support for the named interface.
    #[must_use]
    pub fn has_interface(&self, interface: &str) -> bool {
        self.0
            .handshake_resp
            .negotiated_interfaces
            .contains_key(interface)
    }

    /// Returns the version of the named interface if it's supported on this
    /// connection. Otherwise returns `None`.
    #[must_use]
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
    ///
    /// # Panics
    ///
    /// Will panic if an internal Mutex is poisoned.
    #[must_use]
    pub fn last_serial(&self) -> u32 {
        *self.0.last_serial.lock().unwrap()
    }

    /// Increments the current serial and runs the provided callback with it.
    ///
    /// # Panics
    ///
    /// Will panic if an internal Mutex is poisoned.
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
    ///
    /// # Panics
    ///
    /// Will panic if an internal Mutex is poisoned.
    #[must_use]
    pub fn add_seat(&self, name: Option<&str>, capabilities: BitFlags<DeviceCapability>) -> Seat {
        let seat_version = self.interface_version(eis::Seat::NAME).unwrap_or(1);
        let seat = self.connection().seat(seat_version);
        if let Some(name) = name {
            seat.name(name);
        }

        for capability in capabilities {
            let interface_name = capability.interface_name();

            if !self.has_interface(interface_name) {
                // Not negotiated
                continue;
            }

            // Using bitflag value because as the server we control its meaning
            seat.capability(capability as u64, interface_name);
        }

        seat.done();
        let seat = Seat(Arc::new(SeatInner {
            seat,
            name: name.map(std::borrow::ToOwned::to_owned),
            handle: Arc::downgrade(&self.0),
            advertised_capabilities: capabilities,
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
#[allow(clippy::cast_sign_loss)] // Monotonic clock never returns negatives
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
    connection: Connection,
}

impl EisRequestConverter {
    /// Creates a new converter.
    #[must_use]
    pub fn new(
        context: &eis::Context,
        handshake_resp: EisHandshakeResp,
        initial_serial: u32,
    ) -> Self {
        Self {
            requests: VecDeque::new(),
            pending_requests: VecDeque::new(),
            connection: Connection(Arc::new(ConnectionInner {
                context: context.clone(),
                handshake_resp,
                seats: Mutex::default(),
                devices: Mutex::default(),
                device_for_interface: Mutex::default(),
                last_serial: Mutex::new(initial_serial),
                disconnected: AtomicBool::new(false),
            })),
        }
    }

    /// Returns a handle to the connection used by this converer.
    #[must_use]
    pub fn handle(&self) -> &Connection {
        &self.connection
    }

    fn queue_frame_event(&mut self, device: &Device) {
        self.queue_request(EisRequest::Frame(Frame {
            time: eis_now(),
            device: device.clone(),
            last_serial: self.connection.last_serial(),
        }));
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

    /// Returns the next queued request if one exists.
    pub fn next_request(&mut self) -> Option<EisRequest> {
        self.requests.pop_front()
    }

    /// Handles a low-level protocol-level [`eis::Request`], possibly converting it into
    /// a high-level [`EisRequest`].
    ///
    /// # Panics
    ///
    /// Will panic if an internal Mutex is poisoned.
    ///
    /// # Errors
    ///
    /// The errors returned are protocol violations.
    pub fn handle_request(&mut self, request: eis::Request) -> Result<(), Error> {
        match request {
            eis::Request::Handshake(_handshake, _request) => {
                return Err(Error::UnexpectedHandshakeEvent);
            }
            eis::Request::Connection(_connection, request) => {
                self.handle_connection_request(request)?;
            }
            eis::Request::Callback(_callback, request) => match request {},
            eis::Request::Pingpong(_ping_pong, request) => match request {
                eis::pingpong::Request::Done { callback_data: _ } => {
                    // TODO
                }
            },
            eis::Request::Seat(seat, request) => self.handle_seat_request(&seat, &request)?,
            eis::Request::Device(device, request) => self.handle_device_request(device, request),
            eis::Request::Keyboard(keyboard, request) => {
                let Some(device) = self.connection.device_for_interface(&keyboard) else {
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
                let Some(device) = self.connection.device_for_interface(&pointer) else {
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
                let Some(device) = self.connection.device_for_interface(&pointer_absolute) else {
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
                self.handle_scroll_request(scroll, request);
            }
            eis::Request::Button(button, request) => {
                let Some(device) = self.connection.device_for_interface(&button) else {
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
                self.handle_touchscreen_request(touchscreen, request)?;
            }
        }

        Ok(())
    }

    fn handle_connection_request(
        &mut self,
        request: eis::connection::Request,
    ) -> Result<(), Error> {
        match request {
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
        }
        Ok(())
    }

    fn handle_seat_request(
        &mut self,
        seat: &eis::Seat,
        request: &eis::seat::Request,
    ) -> Result<(), Error> {
        match request {
            eis::seat::Request::Release => {
                self.connection
                    .with_next_serial(|serial| seat.destroyed(serial));
            }
            eis::seat::Request::Bind { capabilities } => {
                let Some(seat) = self.connection.0.seats.lock().unwrap().get(seat).cloned() else {
                    return Ok(());
                };

                let capabilities = DeviceCapability::from_bits(*capabilities)
                    .map_err(|_err| RequestError::InvalidCapabilities)?;
                if !seat.0.advertised_capabilities.contains(capabilities) {
                    return Err(RequestError::InvalidCapabilities.into());
                }

                self.queue_request(EisRequest::Bind(Bind { seat, capabilities }));
                return Ok(());
            }
        }
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)] // Arguably better code when we don't have to dereference data
    fn handle_device_request(&mut self, device: eis::Device, request: eis::device::Request) {
        let Some(device) = self
            .connection
            .0
            .devices
            .lock()
            .unwrap()
            .get(&device)
            .cloned()
        else {
            return;
        };
        match request {
            eis::device::Request::Release => {}
            eis::device::Request::StartEmulating {
                last_serial,
                sequence,
            } => {
                self.queue_request(EisRequest::DeviceStartEmulating(DeviceStartEmulating {
                    device,
                    last_serial,
                    sequence,
                }));
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

    #[allow(clippy::needless_pass_by_value)] // Arguably better code when we don't have to dereference data
    fn handle_scroll_request(&mut self, scroll: eis::Scroll, request: eis::scroll::Request) {
        let Some(device) = self.connection.device_for_interface(&scroll) else {
            return;
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

    #[allow(clippy::needless_pass_by_value)] // Arguably better code when we don't have to dereference data
    fn handle_touchscreen_request(
        &mut self,
        touchscreen: eis::Touchscreen,
        request: eis::touchscreen::Request,
    ) -> Result<(), Error> {
        let Some(device) = self.connection.device_for_interface(&touchscreen) else {
            return Ok(());
        };
        match request {
            eis::touchscreen::Request::Release => {}
            eis::touchscreen::Request::Down { touchid, x, y } => {
                let mut down_touch_ids = device.0.down_touch_ids.lock().unwrap();
                if down_touch_ids.len() == EIS_MAX_TOUCHES {
                    return Err(RequestError::TooManyTouches.into());
                }
                if !down_touch_ids.insert(touchid) {
                    return Err(RequestError::DuplicatedTouchDown.into());
                }
                drop(down_touch_ids);
                self.queue_request(EisRequest::TouchDown(TouchDown {
                    device,
                    touch_id: touchid,
                    x,
                    y,
                    time: 0,
                }));
            }
            eis::touchscreen::Request::Motion { touchid, x, y } => {
                if device.0.down_touch_ids.lock().unwrap().contains(&touchid) {
                    self.queue_request(EisRequest::TouchMotion(TouchMotion {
                        device,
                        touch_id: touchid,
                        x,
                        y,
                        time: 0,
                    }));
                }
            }
            eis::touchscreen::Request::Up { touchid } => {
                if device.0.down_touch_ids.lock().unwrap().remove(&touchid) {
                    self.queue_request(EisRequest::TouchUp(TouchUp {
                        device,
                        touch_id: touchid,
                        time: 0,
                    }));
                }
            }
            eis::touchscreen::Request::Cancel { touchid } => {
                if touchscreen.version() < 2 {
                    return Err(Error::InvalidInterfaceVersion(
                        "ei_touchscreen",
                        touchscreen.version(),
                    ));
                }
                if device.0.down_touch_ids.lock().unwrap().remove(&touchid) {
                    self.queue_request(EisRequest::TouchCancel(TouchCancel {
                        device,
                        touch_id: touchid,
                        time: 0,
                    }));
                }
            }
        }
        Ok(())
    }
}

struct SeatInner {
    seat: eis::Seat,
    name: Option<String>,
    handle: Weak<ConnectionInner>,
    advertised_capabilities: BitFlags<DeviceCapability>,
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
    #[must_use]
    pub fn eis_seat(&self) -> &eis::Seat {
        &self.0.seat
    }

    // builder pattern?
    /// Adds a device to the connection.
    ///
    /// Capabilities that were not advertised on the seat will be ignored. An interface
    /// will be created for all capabilities that do exist on the seat.
    ///
    /// # Panics
    ///
    /// Will panic if an internal Mutex is poisoned.
    pub fn add_device(
        &self,
        name: Option<&str>,
        device_type: eis::device::DeviceType,
        capabilities: BitFlags<DeviceCapability>,
        // TODO: better solution; keymap, etc.
        before_done_cb: impl for<'a> FnOnce(&'a Device),
    ) -> Device {
        let connection = self.0.handle.upgrade().map(Connection);

        let device_version = connection
            .as_ref()
            .and_then(|c| c.interface_version(eis::Device::NAME))
            .unwrap_or(1);
        let device = self.0.seat.device(device_version);
        if let Some(name) = name {
            device.name(name);
        }
        device.device_type(device_type);
        // TODO
        // dimensions
        // regions; region_mapping_id
        let mut interfaces = HashMap::new();
        for capability in capabilities {
            if !self.0.advertised_capabilities.contains(capability) {
                continue;
            }
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
            interfaces.insert(object.interface().to_owned(), object);
        }

        let device = Device(Arc::new(DeviceInner {
            device,
            seat: self.clone(),
            name: name.map(std::string::ToString::to_string),
            interfaces,
            handle: self.0.handle.clone(),
            down_touch_ids: Mutex::new(HashSet::new()),
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
    ///
    /// # Panics
    ///
    /// Will panic if an internal Mutex is poisoned.
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

/// Trait marking interfaces that can be on devices.
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
    // Applicable only for touch devices
    down_touch_ids: Mutex<HashSet<u32>>,
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
    #[must_use]
    pub fn seat(&self) -> &Seat {
        &self.0.seat
    }

    /// Returns the interface proxy for the underlying `ei_device` object.
    #[must_use]
    pub fn device(&self) -> &eis::Device {
        &self.0.device
    }

    /// Returns the name of the device.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    /// Returns an interface proxy if it is implemented for this device.
    ///
    /// Interfaces of devices are implemented, such that there is one `ei_device` object and other objects (for example `ei_keyboard`) denoting capabilities.
    #[must_use]
    pub fn interface<T: DeviceInterface>(&self) -> Option<T> {
        self.0.interfaces.get(T::NAME)?.clone().downcast()
    }

    /// Returns `true` if this device has an interface matching the provided capability.
    #[must_use]
    pub fn has_capability(&self, capability: DeviceCapability) -> bool {
        self.0.interfaces.contains_key(capability.interface_name())
    }

    /// Removes this device and associated interfaces from the connection.
    ///
    /// # Panics
    ///
    /// Will panic if an internal Mutex is poisoned.
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
            handle.with_next_serial(|serial| self.device().resumed(serial));
        }
    }

    /// Notifies to the client that, depending on the context type, no further input events
    /// will be accepted for emulation or no further input events will be sent.
    ///
    /// See [`eis::Device::paused`] for documentation from the protocol specification.
    ///
    /// # Panics
    ///
    /// Will panic if an internal Mutex is poisoned.
    pub fn paused(&self) {
        if let Some(handle) = self.0.handle.upgrade().map(Connection) {
            handle.with_next_serial(|serial| self.device().paused(serial));
        }
        self.0.down_touch_ids.lock().unwrap().clear();
    }

    // TODO: statically restrict the below to receiver context?

    /// Notifies the client that the given device is about to start sending events.
    ///
    /// **Note:** Must only be sent in a receiver context.
    ///
    /// See [`eis::Device::start_emulating`] for documentation from the protocol specification.
    pub fn start_emulating(&self, sequence: u32) {
        if let Some(handle) = self.0.handle.upgrade().map(Connection) {
            handle.with_next_serial(|serial| self.device().start_emulating(serial, sequence));
        }
    }

    /// Notifies the client that the given device is no longer sending events.
    ///
    /// **Note:** Must only be sent in a receiver context.
    ///
    /// See [`eis::Device::stop_emulating`] for documentation from the protocol specification.
    pub fn stop_emulating(&self) {
        if let Some(handle) = self.0.handle.upgrade().map(Connection) {
            handle.with_next_serial(|serial| self.device().stop_emulating(serial));
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
            handle.with_next_serial(|serial| self.device().frame(serial, time));
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
#[allow(missing_docs)] // Inner types have docs
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
    #[must_use]
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
    /// Capabilities requested by the client.
    pub capabilities: BitFlags<DeviceCapability>,
}

/// High-level translation of [`ei_device.frame`](eis::device::Request::Frame).
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Last serial sent by the EIS implementation.
    pub last_serial: u32,
    /// Timestamp in microseconds.
    pub time: u64,
}

/// High-level translation of [`ei_device.start_emulating`](eis::device::Request::StartEmulating).
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStartEmulating {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Last serial sent by the EIS implementation.
    pub last_serial: u32,
    /// The event's sequence number.
    pub sequence: u32,
}

/// High-level translation of [`ei_device.stop_emulating`](eis::device::Request::StopEmulating).
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStopEmulating {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Last serial sent by the EIS implementation.
    pub last_serial: u32,
}

/// High-level translation of [`ei_pointer.motion_relative`](eis::pointer::Request::MotionRelative).
#[derive(Clone, Debug, PartialEq)]
pub struct PointerMotion {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Relative motion on the X axis.
    pub dx: f32,
    /// Relative motion on the Y axis.
    pub dy: f32,
}

/// High-level translation of [`ei_pointer_absolute.motion_absolute`](eis::pointer_absolute::Request::MotionAbsolute).
#[derive(Clone, Debug, PartialEq)]
pub struct PointerMotionAbsolute {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Absolute position on the X axis.
    pub dx_absolute: f32,
    /// Absolute position on the Y axis.
    pub dy_absolute: f32,
}

/// High-level translation of [`ei_button.button`](eis::button::Request::Button).
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Button code, as in Linux's `input-event-codes.h`.
    pub button: u32,
    /// State of the button.
    pub state: eis::button::ButtonState,
}

/// High-level translation of [`ei_scroll.scroll`](eis::scroll::Request::Scroll).
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDelta {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Motion on the X axis.
    pub dx: f32,
    /// Motion on the Y axis.
    pub dy: f32,
}

/// High-level translation of [`ei_scroll.scroll_stop`](eis::scroll::Request::ScrollStop) when its `is_cancel` is zero.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollStop {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Whether motion on the X axis stopped.
    pub x: bool,
    /// Whether motion on the Y axis stopped.
    pub y: bool,
}

/// High-level translation of [`ei_scroll.scroll_stop`](eis::scroll::Request::ScrollStop) when its `is_cancel` is nonzero.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollCancel {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Whether motion on the X axis was canceled.
    pub x: bool,
    /// Whether motion on the Y axis was canceled.
    pub y: bool,
}

/// High-level translation of [`ei_scroll.scroll_discrete`](eis::scroll::Request::ScrollDiscrete).
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDiscrete {
    /// High-level [`Device`] wrapper.
    pub device: Device,
    /// Timestamp in microseconds.
    pub time: u64,
    /// Discrete motion on the X axis.
    pub discrete_dx: i32,
    /// Discrete motion on the Y axis.
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
    /// Absolute position on the X axis.
    pub x: f32,
    /// Absolute position on the Y axis.
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
    /// Absolute position on the X axis.
    pub x: f32,
    /// Absolute position on the Y axis.
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

impl SeatEvent for Bind {
    fn seat(&self) -> &Seat {
        &self.seat
    }
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
