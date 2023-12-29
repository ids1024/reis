// TODO unfinished and unused
#![allow(dead_code)]

use crate::eis;
use std::{collections::VecDeque, fmt, sync::Arc};

pub enum Error {
    UnexpectedHandshakeEvent,
}

#[derive(Default)]
pub struct EisRequestConverter {
    requests: VecDeque<EisRequest>,
    pending_requests: VecDeque<EisRequest>,
}

impl EisRequestConverter {
    pub fn handle_request(&mut self, request: eis::Request) -> Result<(), Error> {
        match request {
            eis::Request::Handshake(_handshake, _request) => {
                return Err(Error::UnexpectedHandshakeEvent);
            }
            eis::Request::Connection(_connection, _request) => {
            },
            eis::Request::Callback(_callback, request) => match request {
            },
            eis::Request::Pingpong(_ping_pong, request) => match request {
                eis::pingpong::Request::Done { callback_data: _ } => {
                    // TODO
                }
            },
            eis::Request::Seat(_seat, _request) => {
            },
            eis::Request::Device(_device, _request) => {
            },
            eis::Request::Keyboard(_keyboard, _request) => {
            },
            eis::Request::Pointer(_pointer, _request) => {
            }
            eis::Request::PointerAbsolute(_pointer_absolute, _request) => {
            }
            eis::Request::Scroll(_scroll, _request) => {
            }
            eis::Request::Button(_button, _request) => {
            }
            eis::Request::Touchscreen(_touchscreen, _request) => {
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

struct DeviceInner {
    device: eis::Device,
    seat: Seat,
    name: Option<String>,
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
