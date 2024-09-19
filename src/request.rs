#![allow(clippy::derive_partial_eq_without_eq)]

// TODO: rename/reorganize things; doc comments on public types/methods

use crate::{eis, wire::Interface, Object, ParseError};
use std::{
    collections::{HashMap, VecDeque},
    fmt, io,
    sync::Arc,
};

pub use crate::event::DeviceCapability;

#[derive(Debug)]
pub enum Error {
    UnexpectedHandshakeEvent,
    UnrecognizedSeat,
    UnrecognizedDevice,
    InvalidCallbackVersion,
    Parse(ParseError),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UnexpectedHandshakeEvent => write!(f, "unexpected handshake event"),
            Self::UnrecognizedSeat => write!(f, "unrecognized seat"),
            Self::UnrecognizedDevice => write!(f, "unrecognized device"),
            Self::InvalidCallbackVersion => write!(f, "invalid callback version"),
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Parse(err) => write!(f, "parse error: {}", err),
        }
    }
}

impl std::error::Error for Error {}

// need way to add seat/device?
#[derive(Debug)]
pub struct EisRequestConverter {
    connection: eis::Connection,
    requests: VecDeque<EisRequest>,
    pending_requests: VecDeque<EisRequest>,
    seats: HashMap<eis::Seat, Seat>,
    devices: HashMap<eis::Device, Device>,
    device_for_interface: HashMap<Object, Device>,
    last_serial: u32,
}

impl EisRequestConverter {
    pub fn new(connection: &eis::Connection, initial_serial: u32) -> Self {
        Self {
            connection: connection.clone(),
            last_serial: initial_serial,
            requests: VecDeque::new(),
            pending_requests: VecDeque::new(),
            seats: HashMap::new(),
            devices: HashMap::new(),
            device_for_interface: HashMap::new(),
        }
    }

    pub fn last_serial(&self) -> u32 {
        self.last_serial
    }

    pub fn next_serial(&mut self) -> u32 {
        self.last_serial += 1;
        self.last_serial
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
            // TODO: If a device request, queue a frame if anything is pending
            self.requests.push_back(request);
        }
    }

    pub fn next_request(&mut self) -> Option<EisRequest> {
        self.requests.pop_front()
    }

    pub fn add_seat(&mut self, name: Option<&str>, capabilities: &[DeviceCapability]) -> Seat {
        let seat = self.connection.seat(1);
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
        }));
        self.seats.insert(seat.0.seat.clone(), seat.clone());
        seat
    }

    // builder pattern?
    pub fn add_device(
        &mut self,
        seat: &Seat,
        name: Option<&str>,
        device_type: eis::device::DeviceType,
        capabilities: &[DeviceCapability],
        // TODO: better solution; keymap, etc.
        before_done_cb: impl for<'a> FnOnce(&'a Device),
    ) -> Device {
        let device = seat.0.seat.device(1);
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
                DeviceCapability::Pointer => device.interface::<eis::Pointer>(1).0,
                DeviceCapability::PointerAbsolute => device.interface::<eis::PointerAbsolute>(1).0,
                DeviceCapability::Keyboard => device.interface::<eis::Keyboard>(1).0,
                DeviceCapability::Touch => device.interface::<eis::Touchscreen>(1).0,
                DeviceCapability::Scroll => device.interface::<eis::Scroll>(1).0,
                DeviceCapability::Button => device.interface::<eis::Button>(1).0,
            };
            interfaces.insert(object.interface().to_string(), object);
        }

        let device = Device(Arc::new(DeviceInner {
            device,
            seat: seat.clone(),
            name: name.map(|x| x.to_string()),
            interfaces,
        }));
        for interface in device.0.interfaces.values() {
            self.device_for_interface
                .insert(interface.clone(), device.clone());
        }
        self.devices.insert(device.0.device.clone(), device.clone());

        before_done_cb(&device);
        device.device().done();

        device
    }

    pub fn handle_request(&mut self, request: eis::Request) -> Result<(), Error> {
        match request {
            eis::Request::Handshake(_handshake, _request) => {
                return Err(Error::UnexpectedHandshakeEvent);
            }
            eis::Request::Connection(_connection, request) => match request {
                eis::connection::Request::Sync { callback } => {
                    if callback.version() != 1 {
                        return Err(Error::InvalidCallbackVersion);
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
                    // XXX
                    let serial = self.next_serial();
                    seat.destroyed(serial);
                }
                eis::seat::Request::Bind { capabilities } => {
                    let seat = self.seats.get(&seat).ok_or(Error::UnrecognizedSeat)?;
                    self.queue_request(EisRequest::Bind(Bind {
                        seat: seat.clone(),
                        capabilities,
                    }));
                }
            },
            eis::Request::Device(device, request) => {
                let device = self.devices.get(&device).ok_or(Error::UnrecognizedDevice)?;
                match request {
                    eis::device::Request::Release => {}
                    eis::device::Request::StartEmulating {
                        last_serial,
                        sequence,
                    } => {
                        self.queue_request(EisRequest::DeviceStartEmulating(
                            DeviceStartEmulating {
                                device: device.clone(),
                                last_serial,
                                sequence,
                            },
                        ));
                    }
                    eis::device::Request::StopEmulating { last_serial } => {
                        self.queue_request(EisRequest::DeviceStopEmulating(DeviceStopEmulating {
                            device: device.clone(),
                            last_serial,
                        }));
                    }
                    eis::device::Request::Frame {
                        last_serial,
                        timestamp,
                    } => {
                        self.queue_request(EisRequest::Frame(Frame {
                            device: device.clone(),
                            last_serial,
                            time: timestamp,
                        }));
                    }
                }
            }
            eis::Request::Keyboard(keyboard, request) => {
                let device = self
                    .device_for_interface
                    .get(&keyboard.0)
                    .ok_or(Error::UnrecognizedDevice)?;
                match request {
                    eis::keyboard::Request::Release => {}
                    eis::keyboard::Request::Key { key, state } => {
                        self.queue_request(EisRequest::KeyboardKey(KeyboardKey {
                            device: device.clone(),
                            key,
                            state,
                            time: 0,
                        }));
                    }
                }
            }
            eis::Request::Pointer(pointer, request) => {
                let device = self
                    .device_for_interface
                    .get(&pointer.0)
                    .ok_or(Error::UnrecognizedDevice)?;
                match request {
                    eis::pointer::Request::Release => {}
                    eis::pointer::Request::MotionRelative { x, y } => {
                        self.queue_request(EisRequest::PointerMotion(PointerMotion {
                            device: device.clone(),
                            dx: x,
                            dy: y,
                            time: 0,
                        }));
                    }
                }
            }
            eis::Request::PointerAbsolute(pointer_absolute, request) => {
                let device = self
                    .device_for_interface
                    .get(&pointer_absolute.0)
                    .ok_or(Error::UnrecognizedDevice)?;
                match request {
                    eis::pointer_absolute::Request::Release => {}
                    eis::pointer_absolute::Request::MotionAbsolute { x, y } => {
                        self.queue_request(EisRequest::PointerMotionAbsolute(
                            PointerMotionAbsolute {
                                device: device.clone(),
                                dx_absolute: x,
                                dy_absolute: y,
                                time: 0,
                            },
                        ));
                    }
                }
            }
            eis::Request::Scroll(scroll, request) => {
                let device = self
                    .device_for_interface
                    .get(&scroll.0)
                    .ok_or(Error::UnrecognizedDevice)?;
                match request {
                    eis::scroll::Request::Release => {}
                    eis::scroll::Request::Scroll { x, y } => {
                        self.queue_request(EisRequest::ScrollDelta(ScrollDelta {
                            device: device.clone(),
                            dx: x,
                            dy: y,
                            time: 0,
                        }));
                    }
                    eis::scroll::Request::ScrollDiscrete { x, y } => {
                        self.queue_request(EisRequest::ScrollDiscrete(ScrollDiscrete {
                            device: device.clone(),
                            discrete_dx: x,
                            discrete_dy: y,
                            time: 0,
                        }));
                    }
                    eis::scroll::Request::ScrollStop { x, y, is_cancel } => {
                        if is_cancel != 0 {
                            self.queue_request(EisRequest::ScrollCancel(ScrollCancel {
                                device: device.clone(),
                                time: 0,
                                x: x != 0,
                                y: y != 0,
                            }));
                        } else {
                            self.queue_request(EisRequest::ScrollStop(ScrollStop {
                                device: device.clone(),
                                time: 0,
                                x: x != 0,
                                y: y != 0,
                            }));
                        }
                    }
                }
            }
            eis::Request::Button(button, request) => {
                let device = self
                    .device_for_interface
                    .get(&button.0)
                    .ok_or(Error::UnrecognizedDevice)?;
                match request {
                    eis::button::Request::Release => {}
                    eis::button::Request::Button { button, state } => {
                        self.queue_request(EisRequest::Button(Button {
                            device: device.clone(),
                            button,
                            state,
                            time: 0,
                        }));
                    }
                }
            }
            eis::Request::Touchscreen(touchscreen, request) => {
                let device = self
                    .device_for_interface
                    .get(&touchscreen.0)
                    .ok_or(Error::UnrecognizedDevice)?;
                match request {
                    eis::touchscreen::Request::Release => {}
                    eis::touchscreen::Request::Down { touchid, x, y } => {
                        self.queue_request(EisRequest::TouchDown(TouchDown {
                            device: device.clone(),
                            touch_id: touchid,
                            x,
                            y,
                            time: 0,
                        }));
                    }
                    eis::touchscreen::Request::Motion { touchid, x, y } => {
                        self.queue_request(EisRequest::TouchMotion(TouchMotion {
                            device: device.clone(),
                            touch_id: touchid,
                            x,
                            y,
                            time: 0,
                        }));
                    }
                    eis::touchscreen::Request::Up { touchid } => {
                        self.queue_request(EisRequest::TouchUp(TouchUp {
                            device: device.clone(),
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
}

#[derive(Clone)]
pub struct Seat(Arc<SeatInner>);

impl Seat {
    pub fn eis_seat(&self) -> &eis::Seat {
        &self.0.seat
    }
}

impl fmt::Debug for Seat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = &self.0.name {
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

#[allow(dead_code)]
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

    pub fn device(&self) -> &eis::Device {
        &self.0.device
    }

    pub fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    pub fn interface<T: DeviceInterface>(&self) -> Option<T> {
        self.0.interfaces.get(T::NAME)?.clone().downcast()
    }

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
}

impl EisRequest {
    // Requests that are grouped by frames need their times set when the
    // frame request occurs.
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
pub struct Bind {
    pub seat: Seat,
    pub capabilities: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub device: Device,
    pub last_serial: u32,
    pub time: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStartEmulating {
    pub device: Device,
    pub last_serial: u32,
    pub sequence: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStopEmulating {
    pub device: Device,
    pub last_serial: u32,
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
    pub state: eis::button::ButtonState,
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
    pub state: eis::keyboard::KeyState,
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
