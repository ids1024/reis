use ashpd::desktop::remote_desktop::{DeviceType, RemoteDesktop};
use calloop::generic::Generic;
use once_cell::sync::Lazy;
use reis::{ei, PendingRequestResult};
use std::{
    collections::HashMap,
    io,
    os::unix::{io::FromRawFd, net::UnixStream},
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
    m
});

#[derive(Debug, Default)]
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
    // XXX best way to handle data associated with object?
    seats: HashMap<ei::Seat, SeatData>,
    // XXX association with seat?
    devices: HashMap<ei::Device, DeviceData>,
    running: bool,
}

impl State {
    fn handle_listener_readable(
        &mut self,
        context: &mut ei::Context,
    ) -> io::Result<calloop::PostAction> {
        match context.read() {
            Ok(res) if res.is_eof() => {
                return Ok(calloop::PostAction::Remove);
            }
            Err(_) => {
                return Ok(calloop::PostAction::Remove);
            }
            _ => {}
        }

        while let Some(result) = context.pending_event() {
            let request = match result {
                PendingRequestResult::Request(request) => request,
                PendingRequestResult::ParseError(msg) => {
                    todo!()
                }
                PendingRequestResult::InvalidObject(object_id) => {
                    // TODO
                    continue;
                }
            };
            match request {
                ei::Event::Handshake(handshake, request) => match request {
                    ei::handshake::Event::HandshakeVersion { version: _ } => {
                        handshake.handshake_version(1);
                        handshake.name("type-text-example");
                        handshake.context_type(ei::handshake::ContextType::Sender);
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
                            seat.bind(*data.capabilities.get("ei_keyboard").unwrap());
                            // XXX
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
                        ei::device::Event::Done => {
                            if let Some(keyboard) = data.interface::<ei::Keyboard>() {
                                let context = xkb::Context::new(0);
                                let keymap =
                                    xkb::Keymap::new_from_names(&context, "", "", "", "", None, 0)
                                        .unwrap();
                                let s = "Hello world!";
                                for c in s.chars() {
                                    let keysym = xkb::Keysym::from_char(c);
                                    let mut keycode = None;
                                    'outer: for i in
                                        keymap.min_keycode().raw()..keymap.max_keycode().raw()
                                    {
                                        for j in 0..=1 {
                                            let syms = keymap.key_get_syms_by_level(
                                                xkb::Keycode::new(i),
                                                0,
                                                j,
                                            );
                                            if syms.contains(&keysym) {
                                                keycode = Some(i);
                                                break 'outer;
                                            }
                                        }
                                    }
                                    let keycode = keycode.unwrap();
                                    keyboard.key(keycode, ei::keyboard::KeyState::Press);
                                    keyboard.key(keycode, ei::keyboard::KeyState::Released);
                                }
                                self.running = false;
                            }
                        }
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
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        context.flush();

        Ok(calloop::PostAction::Continue)
    }
}

async fn open_connection() -> ei::Context {
    if let Some(context) = ei::Context::connect_to_env().unwrap() {
        context
    } else {
        eprintln!("Unable to find ei socket. Trying xdg desktop portal.");
        let remote_desktop = RemoteDesktop::new().await.unwrap();
        let session = remote_desktop.create_session().await.unwrap();
        remote_desktop
            .select_devices(&session, DeviceType::Keyboard.into())
            .await
            .unwrap();
        // XXX window identifier?
        remote_desktop
            .start(&session, &ashpd::WindowIdentifier::from_xid(0))
            .await
            .unwrap();
        let raw_fd = remote_desktop.connect_to_eis(&session).await.unwrap();
        let stream = unsafe { UnixStream::from_raw_fd(raw_fd) };
        ei::Context::new(stream).unwrap()
    }
}

fn main() {
    let mut event_loop = calloop::EventLoop::try_new().unwrap();
    let handle = event_loop.handle();

    let context = futures_executor::block_on(open_connection());
    // XXX wait for server version?
    let handshake = context.handshake();
    context.flush();
    let context_source = Generic::new(context, calloop::Interest::READ, calloop::Mode::Level);
    handle
        .insert_source(context_source, |_event, context, state: &mut State| {
            state.handle_listener_readable(unsafe { context.get_mut() })
        })
        .unwrap();

    let mut state = State {
        seats: HashMap::new(),
        devices: HashMap::new(),
        running: true,
    };
    while state.running {
        event_loop.dispatch(None, &mut state).unwrap();
    }
}
