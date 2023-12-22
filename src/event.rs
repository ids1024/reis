// libei has user_data for seat, device, etc. Do we need that?

// XXX device_for_interface may be empty if something is sent before done. Can any event be sent
// then?

#![allow(clippy::single_match)]

use crate::{ei, Interface, Object};
use std::{
    collections::{HashMap, VecDeque},
    os::unix::io::OwnedFd,
    sync::Arc,
};

// struct ReceiverStream(EiEventStream, ());

#[derive(Debug)]
pub enum Error {
    DeviceEventBeforeDone,
    DeviceEventAfterDone,
    NoDeviceType,
}

#[derive(Default)]
pub struct EiEventConverter {
    pending_seats: HashMap<ei::Seat, SeatInner>,
    seats: HashMap<ei::Seat, Seat>,
    pending_devices: HashMap<ei::Device, DeviceInner>,
    devices: HashMap<ei::Device, Device>,
    device_for_interface: HashMap<Object, Device>,
    events: VecDeque<EiEvent>,
    pending_events: VecDeque<EiEvent>,
}

impl EiEventConverter {
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
            ei::Event::Connection(_connection, request) => match request {
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
                }
                _ => {}
            },
            ei::Event::Seat(seat, request) => {
                match request {
                    ei::seat::Event::Name { name } => {
                        // XXX error
                        let seat = self.pending_seats.get_mut(&seat).unwrap();
                        seat.name = Some(name);
                    }
                    ei::seat::Event::Capability { mask, interface } => {
                        // XXX error
                        let seat = self.pending_seats.get_mut(&seat).unwrap();
                        seat.capabilities.insert(interface, mask);
                    }
                    ei::seat::Event::Done => {
                        // let caps = data.capabilities.values().fold(0, |a, b| a | b);
                        // seat.bind(caps);
                        // XXX error
                        let seat = self.pending_seats.remove(&seat).unwrap();
                        let seat = Seat(Arc::new(seat));
                        self.seats.insert(seat.0.seat.clone(), seat.clone());
                        self.queue_event(EiEvent::SeatAdded(SeatAdded { seat }));
                    }
                    ei::seat::Event::Device { device } => {
                        // XXX error
                        let seat = self.seats.get_mut(&seat).unwrap();
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
                    _ => {}
                }
            }
            ei::Event::Device(device, request) => {
                match request {
                    ei::device::Event::Name { name } => {
                        // XXX error
                        let device = self.pending_devices.get_mut(&device).unwrap();
                        device.name = Some(name);
                    }
                    ei::device::Event::DeviceType { device_type } => {
                        // XXX error
                        let device = self.pending_devices.get_mut(&device).unwrap();
                        device.device_type = Some(device_type);
                    }
                    ei::device::Event::Interface { object } => {
                        // XXX error
                        let device = self.pending_devices.get_mut(&device).unwrap();
                        device
                            .interfaces
                            .insert(object.interface().to_string(), object);
                    }
                    ei::device::Event::Dimensions { width, height } => {
                        // XXX error
                        let device = self.pending_devices.get_mut(&device).unwrap();
                        device.dimensions = Some((width, height));
                    }
                    ei::device::Event::Region {
                        offset_x,
                        offset_y,
                        width,
                        hight,
                        scale,
                    } => {
                        // XXX error
                        let device = self.pending_devices.get_mut(&device).unwrap();
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
                        // XXX error
                        let device = self.pending_devices.remove(&device).unwrap();
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
                        // XXX error
                        let device = self.pending_devices.get_mut(&device).unwrap();
                        device.next_region_mapping_id = Some(mapping_id);
                    }
                    ei::device::Event::Frame { serial, timestamp } => {
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
                    _ => {}
                }
            }
            ei::Event::Keyboard(keyboard, request) => {
                match request {
                    ei::keyboard::Event::Keymap {
                        keymap_type,
                        size,
                        keymap,
                    } => {
                        // XXX
                        let device = self
                            .pending_devices
                            .values_mut()
                            .find(|i| i.interfaces.values().any(|j| j == &keyboard.0))
                            .unwrap();
                        device.keymap = Some(Keymap {
                            type_: keymap_type,
                            size,
                            fd: keymap,
                        });
                    }
                    ei::keyboard::Event::Key { key, state } => {
                        let device = self.device_for_interface.get(&keyboard.0).unwrap();
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
                        let device = self.device_for_interface.get(&keyboard.0).unwrap();
                        self.queue_event(EiEvent::KeyboardModifiers(KeyboardModifiers {
                            device: device.clone(),
                            serial,
                            depressed,
                            locked,
                            latched,
                            group,
                        }));
                    }
                    _ => {}
                }
            }
            ei::Event::Pointer(pointer, request) => {
                let device = self.device_for_interface.get(&pointer.0).unwrap();
                match request {
                    ei::pointer::Event::MotionRelative { x, y } => {
                        self.queue_event(EiEvent::PointerMotion(PointerMotion {
                            device: device.clone(),
                            time: 0,
                            dx: x,
                            dy: y,
                        }));
                    }
                    _ => {}
                }
            }
            ei::Event::PointerAbsolute(pointer_absolute, request) => {
                let device = self.device_for_interface.get(&pointer_absolute.0).unwrap();
                match request {
                    ei::pointer_absolute::Event::MotionAbsolute { x, y } => {
                        self.queue_event(EiEvent::PointerMotionAbsolute(PointerMotionAbsolute {
                            device: device.clone(),
                            time: 0,
                            dx_absolute: x,
                            dy_absolute: y,
                        }));
                    }
                    _ => {}
                }
            }
            ei::Event::Scroll(scroll, request) => {
                let device = self.device_for_interface.get(&scroll.0).unwrap();
                match request {
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
                    _ => {}
                }
            }
            ei::Event::Button(button, request) => {
                let device = self.device_for_interface.get(&button.0).unwrap();
                match request {
                    ei::button::Event::Button { button, state } => {
                        self.queue_event(EiEvent::Button(Button {
                            device: device.clone(),
                            time: 0,
                            button,
                            state,
                        }));
                    }
                    _ => {}
                }
            }
            ei::Event::Touchscreen(touchscreen, request) => {
                let device = self.device_for_interface.get(&touchscreen.0).unwrap();
                match request {
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
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn next_event(&mut self) -> Option<EiEvent> {
        self.events.pop_front()
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
pub enum DeviceCapability {
    Pointer,
    PointerAbsolute,
    Keyboard,
    Touch,
    Scroll,
    Button,
}

#[derive(Debug)]
struct SeatInner {
    seat: ei::Seat,
    name: Option<String>,
    capabilities: HashMap<String, u64>,
}

#[derive(Clone, Debug)]
pub struct Seat(Arc<SeatInner>);

impl PartialEq for Seat {
    fn eq(&self, rhs: &Seat) -> bool {
        Arc::ptr_eq(&self.0, &rhs.0)
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
            let name = match i {
                DeviceCapability::Pointer => ei::Pointer::NAME,
                DeviceCapability::PointerAbsolute => ei::PointerAbsolute::NAME,
                DeviceCapability::Keyboard => ei::Keyboard::NAME,
                DeviceCapability::Touch => ei::Touchscreen::NAME,
                DeviceCapability::Scroll => ei::Scroll::NAME,
                DeviceCapability::Button => ei::Button::NAME,
            };
            if let Some(value) = self.0.capabilities.get(name) {
                caps |= value;
            }
        }
        self.0.seat.bind(caps);
    }

    // TODO: mirror C API more?
    // fn unbind_capabilities() {}
}

#[derive(Debug)]
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

#[derive(Clone, Debug)]
pub struct Device(Arc<DeviceInner>);

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
