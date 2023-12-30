// TODO unfinished and unused
#![allow(dead_code)]

// TODO: rename/reorganize things; doc comments on public types/methods

use crate::{eis, Object};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    sync::Arc,
};

pub use crate::event::DeviceCapability;

pub enum Error {
    UnexpectedHandshakeEvent,
    UnrecognizedSeat,
    UnrecognizedDevice,
}

// need way to add seat/device?
pub struct EisRequestConverter {
    connection: eis::Connection,
    requests: VecDeque<EisRequest>,
    pending_requests: VecDeque<EisRequest>,
    seats: HashMap<eis::Seat, Seat>,
    devices: HashMap<eis::Device, Device>,
    device_for_interface: HashMap<Object, Device>,
}

impl EisRequestConverter {
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

    pub fn add_seat(&mut self, name: Option<&str>) -> Seat {
        let seat = self.connection.seat(1);
        if let Some(name) = name {
            seat.name(name);
        }
        // TODO capabilities
        seat.done();
        let seat = Seat(Arc::new(SeatInner {
            seat,
            name: name.map(|x| x.to_owned()),
        }));
        self.seats.insert(seat.0.seat.clone(), seat.clone());
        seat
    }

    pub fn handle_request(&mut self, request: eis::Request) -> Result<(), Error> {
        match request {
            eis::Request::Handshake(_handshake, _request) => {
                return Err(Error::UnexpectedHandshakeEvent);
            }
            eis::Request::Connection(_connection, request) => match request {
                eis::connection::Request::Sync { callback } => {
                    callback.done(0);
                    if let Some(backend) = callback.0.backend() {
                        // XXX Error?
                        let _ = backend.flush();
                    }
                }
                eis::connection::Request::Disconnect => {}
            },
            eis::Request::Callback(_callback, request) => match request {},
            eis::Request::Pingpong(_ping_pong, request) => match request {
                eis::pingpong::Request::Done { callback_data: _ } => {
                    // TODO
                }
            },
            eis::Request::Seat(seat, request) => match request {
                eis::seat::Request::Release => {}
                eis::seat::Request::Bind { capabilities: _ } => {
                    let _seat = self.seats.get(&seat).ok_or(Error::UnrecognizedSeat)?;
                    // TODO bind event
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
            eis::Request::Keyboard(_keyboard, request) => match request {
                eis::keyboard::Request::Release => {}
                eis::keyboard::Request::Key { key: _, state: _ } => {}
            },
            eis::Request::Pointer(_pointer, request) => match request {
                eis::pointer::Request::Release => {}
                eis::pointer::Request::MotionRelative { x: _, y: _ } => {}
            },
            eis::Request::PointerAbsolute(_pointer_absolute, request) => match request {
                eis::pointer_absolute::Request::Release => {}
                eis::pointer_absolute::Request::MotionAbsolute { x: _, y: _ } => {}
            },
            eis::Request::Scroll(_scroll, request) => match request {
                eis::scroll::Request::Release => {}
                eis::scroll::Request::Scroll { x: _, y: _ } => {}
                eis::scroll::Request::ScrollDiscrete { x: _, y: _ } => {}
                eis::scroll::Request::ScrollStop {
                    x: _,
                    y: _,
                    is_cancel: _,
                } => {}
            },
            eis::Request::Button(_button, request) => match request {
                eis::button::Request::Release => {}
                eis::button::Request::Button {
                    button: _,
                    state: _,
                } => {}
            },
            eis::Request::Touchscreen(_touchscreen, request) => match request {
                eis::touchscreen::Request::Release => {}
                eis::touchscreen::Request::Down {
                    touchid: _,
                    x: _,
                    y: _,
                } => {}
                eis::touchscreen::Request::Motion {
                    touchid: _,
                    x: _,
                    y: _,
                } => {}
                eis::touchscreen::Request::Up { touchid: _ } => {}
            },
        }
        Ok(())
    }
}

struct SeatInner {
    seat: eis::Seat,
    name: Option<String>,
    //capabilities: HashMap<String, u64>,
}
// Method to add device to seat?
// eis_seat_new_device

#[derive(Clone)]
pub struct Seat(Arc<SeatInner>);

impl Seat {
    // builder pattern?
    pub fn add_device(
        &self,
        name: Option<&str>,
        device_type: eis::device::DeviceType,
        capabilities: &[DeviceCapability],
    ) -> Device {
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
                DeviceCapability::Pointer => device.interface::<eis::Pointer>(1).0,
                DeviceCapability::PointerAbsolute => device.interface::<eis::PointerAbsolute>(1).0,
                DeviceCapability::Keyboard => device.interface::<eis::Keyboard>(1).0,
                DeviceCapability::Touch => device.interface::<eis::Touchscreen>(1).0,
                DeviceCapability::Scroll => device.interface::<eis::Scroll>(1).0,
                DeviceCapability::Button => device.interface::<eis::Button>(1).0,
            };
            interfaces.insert(object.interface().to_string(), object);
        }
        device.done();

        let device = Device(Arc::new(DeviceInner {
            device,
            seat: self.clone(),
            name: name.map(|x| x.to_string()),
            interfaces,
        }));
        // TODO insert into device list used in converter
        // self.devices.insert(device.0.device.clone(), device.clone());
        device
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
}

impl PartialEq for Device {
    fn eq(&self, rhs: &Device) -> bool {
        Arc::ptr_eq(&self.0, &rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EisRequest {
    // TODO connect, disconnect, seat bind, device closed
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
