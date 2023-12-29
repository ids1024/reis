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
            eis::Request::Seat(_seat, request) => match request {
                eis::seat::Request::Release => {}
                eis::seat::Request::Bind { capabilities: _ } => {}
            },
            eis::Request::Device(_device, request) => match request {
                eis::device::Request::Release => {}
                eis::device::Request::StartEmulating {
                    last_serial: _,
                    sequence: _,
                } => {}
                eis::device::Request::StopEmulating { last_serial: _ } => {}
                eis::device::Request::Frame {
                    last_serial: _,
                    timestamp: _,
                } => {}
            },
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
