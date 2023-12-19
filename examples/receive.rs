use ashpd::desktop::input_capture::{Capabilities, InputCapture};
use futures::stream::StreamExt;
use once_cell::sync::Lazy;
use reis::{ei, tokio::EiEventStream, PendingRequestResult};
use std::{
    collections::HashMap,
    os::unix::{
        io::{AsRawFd, FromRawFd},
        net::UnixStream,
    },
};
use xkbcommon::xkb;

static INTERFACES: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("ei_callback", 1);
    m.insert("ei_connection", 1);
    m.insert("ei_seat", 1);
    m.insert("ei_device", 1);
    m.insert("ei_pingpong", 1);
    m.insert("ei_keyboard", 1);
    m.insert("ei_pointer", 1);
    m.insert("ei_scroll", 1);
    m.insert("ei_touchscreen", 1);
    m
});

#[derive(Default)]
struct SeatData {
    name: Option<String>,
    capabilities: HashMap<String, u64>,
}

#[derive(Default)]
struct DeviceData {
    name: Option<String>,
    device_type: Option<ei::device::DeviceType>,
    interfaces: HashMap<String, reis::Object>,
}

impl DeviceData {
    fn interface<T: reis::Interface>(&self) -> Option<T> {
        self.interfaces.get(T::NAME)?.clone().downcast()
    }
}

struct State {
    context: ei::Context,
    // XXX best way to handle data associated with object?
    seats: HashMap<ei::Seat, SeatData>,
    // XXX association with seat?
    devices: HashMap<ei::Device, DeviceData>,
}

impl State {
    fn handle_event(&mut self, event: ei::Event) {
        match event {
            ei::Event::Handshake(handshake, request) => match request {
                ei::handshake::Event::HandshakeVersion { version: _ } => {
                    handshake.handshake_version(1);
                    handshake.name("receive-example");
                    handshake.context_type(ei::handshake::ContextType::Receiver);
                    for (interface, version) in INTERFACES.iter() {
                        handshake.interface_version(interface, *version);
                    }
                    handshake.finish();
                }
                ei::handshake::Event::InterfaceVersion {
                    name: _,
                    version: _,
                } => {}
                ei::handshake::Event::Connection {
                    connection: _,
                    serial: _,
                } => {}
                _ => {}
            },
            ei::Event::Connection(connection, request) => match request {
                ei::connection::Event::Seat { seat } => {
                    self.seats.insert(seat, Default::default());
                }
                ei::connection::Event::Ping { ping } => {
                    ping.done(0);
                }
                _ => {}
            },
            ei::Event::Seat(seat, request) => {
                let data = self.seats.get_mut(&seat).unwrap();
                match request {
                    ei::seat::Event::Name { name } => {
                        data.name = Some(name);
                    }
                    ei::seat::Event::Capability { mask, interface } => {
                        data.capabilities.insert(interface, mask);
                    }
                    ei::seat::Event::Done => {
                        let caps = data.capabilities.values().fold(0, |a, b| a | b);
                        seat.bind(caps);
                    }
                    ei::seat::Event::Device { device } => {
                        self.devices.insert(device, Default::default());
                    }
                    _ => {}
                }
            }
            ei::Event::Device(device, request) => {
                let data = self.devices.get_mut(&device).unwrap();
                match request {
                    ei::device::Event::Name { name } => {
                        data.name = Some(name);
                    }
                    ei::device::Event::DeviceType { device_type } => {
                        data.device_type = Some(device_type);
                    }
                    ei::device::Event::Interface { object } => {
                        data.interfaces
                            .insert(object.interface().to_string(), object);
                    }
                    ei::device::Event::Done => {}
                    ei::device::Event::Resumed { serial } => {}
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
                        // XXX format
                        // flags?
                        let context = xkb::Context::new(0);
                        let keymap = unsafe {
                            xkb::Keymap::new_from_fd(
                                &context,
                                keymap,
                                size as _,
                                xkb::KEYMAP_FORMAT_TEXT_V1,
                                0,
                            )
                        }
                        .unwrap()
                        .unwrap();
                    }
                    ei::keyboard::Event::Key { key, state } => {
                        println!("Key: {key}");
                    }
                    ei::keyboard::Event::Modifiers {
                        serial,
                        depressed,
                        locked,
                        latched,
                        group,
                    } => {}
                    _ => {}
                }
            }
            _ => {}
        }

        self.context.flush();
    }
}

async fn open_connection() -> ei::Context {
    if let Some(context) = ei::Context::connect_to_env().unwrap() {
        context
    } else {
        eprintln!("Unable to find ei socket. Trying xdg desktop portal.");
        let input_capture = InputCapture::new().await.unwrap();
        // XXX window identifier?
        let session = input_capture
            .create_session(
                &ashpd::WindowIdentifier::from_xid(0),
                (Capabilities::Keyboard | Capabilities::Pointer | Capabilities::Touchscreen).into(),
            )
            .await
            .unwrap()
            .0;
        input_capture.enable(&session).await.unwrap();
        let raw_fd = input_capture.connect_to_eis(&session).await.unwrap();
        let stream = unsafe { UnixStream::from_raw_fd(raw_fd) };
        ei::Context::new(stream).unwrap()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let context = open_connection().await;
    // XXX wait for server version?
    let handshake = context.handshake();
    context.flush();

    let mut state = State {
        context: context.clone(),
        seats: HashMap::new(),
        devices: HashMap::new(),
    };

    let mut events = EiEventStream::new(context.clone()).unwrap();
    while let Some(result) = events.next().await {
        let event = match result.unwrap() {
            PendingRequestResult::Request(event) => event,
            PendingRequestResult::ParseError(msg) => {
                todo!()
            }
            PendingRequestResult::InvalidObject(object_id) => {
                // TODO
                continue;
            }
        };

        state.handle_event(event);
    }
}
