use ashpd::desktop::input_capture::{Barrier, Capabilities, InputCapture};
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
    m.insert("ei_connection", 1);
    m.insert("ei_callback", 1);
    m.insert("ei_pingpong", 1);
    m.insert("ei_seat", 1);
    m.insert("ei_device", 1);
    m.insert("ei_pointer", 1);
    m.insert("ei_pointer_absolute", 1);
    m.insert("ei_scroll", 1);
    m.insert("ei_button", 1);
    m.insert("ei_keyboard", 1);
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
                    ei::device::Event::Frame { serial, timestamp } => {
                        println!("device frame");
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
                        // Escape key
                        if key == 1 {
                            std::process::exit(0);
                        }
                        println!("key {key} {state:?}");
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
            ei::Event::Pointer(pointer, request) => match request {
                ei::pointer::Event::MotionRelative { x, y } => {
                    println!("relative motion {x}, {y}");
                }
                _ => {}
            },
            ei::Event::PointerAbsolute(pointer_absolute, request) => match request {
                ei::pointer_absolute::Event::MotionAbsolute { x, y } => {
                    println!("absolute motion {x}, {y}");
                }
                _ => {}
            },
            ei::Event::Scroll(scroll, request) => match request {
                ei::scroll::Event::Scroll { x, y } => {
                    println!("scroll {x}, {y}");
                }
                ei::scroll::Event::ScrollDiscrete { x, y } => {
                    println!("scroll discrete {x}, {y}");
                }
                ei::scroll::Event::ScrollStop { x, y, is_cancel } => {
                    println!("scroll stop {x}, {y}, {is_cancel}");
                }
                _ => {}
            },
            ei::Event::Button(button, request) => match request {
                ei::button::Event::Button { button, state } => {
                    println!("button {button} {state:?}");
                }
                _ => {}
            },
            ei::Event::Touchscreen(touchscreen, request) => match request {
                ei::touchscreen::Event::Down { touchid, x, y } => {
                    println!("touch down {touchid} {x} {y}");
                }
                ei::touchscreen::Event::Motion { touchid, x, y } => {
                    println!("touch motion {touchid} {x} {y}");
                }
                ei::touchscreen::Event::Up { touchid } => {
                    println!("touch up {touchid}");
                }
                _ => {}
            },
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
        let raw_fd = input_capture.connect_to_eis(&session).await.unwrap();
        let stream = unsafe { UnixStream::from_raw_fd(raw_fd) };
        let zones = input_capture
            .zones(&session)
            .await
            .unwrap()
            .response()
            .unwrap();

        let barriers = zones
            .regions()
            .iter()
            .enumerate()
            .map(|(n, region)| {
                let x = region.x_offset();
                let y = region.y_offset();
                let w = region.width() as i32;
                let h = region.height() as i32;
                Barrier::new(n as u32 + 1, (x, y, x + w - 1, y))
            })
            .collect::<Vec<_>>();
        let resp = input_capture
            .set_pointer_barriers(&session, &barriers, zones.zone_set())
            .await
            .unwrap()
            .response()
            .unwrap();
        assert_eq!(&resp.failed_barriers(), &[]);
        eprintln!("Set capture barrier to top edge of screen.");
        eprintln!("(When input is captured, Esc will exit.)");
        input_capture.enable(&session).await.unwrap();
        ei::Context::new(stream).unwrap()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let context = open_connection().await;
    let mut events = EiEventStream::new(context.clone()).unwrap();
    reis::tokio::ei_handshake(
        &mut events,
        "receive-example",
        ei::handshake::ContextType::Receiver,
        &INTERFACES,
    )
    .await
    .unwrap();

    let mut state = State {
        context: context.clone(),
        seats: HashMap::new(),
        devices: HashMap::new(),
    };

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
