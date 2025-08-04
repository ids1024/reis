//! Typing text.

use ashpd::desktop::{
    remote_desktop::{DeviceType, RemoteDesktop},
    PersistMode,
};
use calloop::generic::Generic;
use once_cell::sync::Lazy;
use reis::{ei, PendingRequestResult};
use std::{collections::HashMap, io, os::unix::net::UnixStream};
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
    sequence: u32,
    last_serial: u32,
    keymap: Option<xkb::Keymap>,
}

impl State {
    #![allow(clippy::unnecessary_wraps, clippy::too_many_lines)]
    fn handle_listener_readable(
        &mut self,
        context: &mut ei::Context,
    ) -> io::Result<calloop::PostAction> {
        if context.read().is_err() {
            return Ok(calloop::PostAction::Remove);
        }

        while let Some(result) = context.pending_event() {
            let request = match result {
                PendingRequestResult::Request(request) => request,
                PendingRequestResult::ParseError(_msg) => {
                    todo!()
                }
                PendingRequestResult::InvalidObject(_object_id) => {
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
                    ei::handshake::Event::Connection {
                        connection: _,
                        serial,
                    } => {
                        self.last_serial = serial;
                    }
                    _ => {}
                },
                ei::Event::Connection(_connection, request) => match request {
                    ei::connection::Event::Seat { seat } => {
                        self.seats.insert(seat, SeatData::default());
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
                            self.devices.insert(device, DeviceData::default());
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
                                .insert(object.interface().to_owned(), object);
                        }
                        ei::device::Event::Done => {
                            if let Some(keyboard) = data.interface::<ei::Keyboard>() {
                                device.start_emulating(self.sequence, self.last_serial);
                                self.sequence += 1;
                                let keymap = self.keymap.as_ref().unwrap();
                                let all_keycodes =
                                    keymap.min_keycode().raw()..keymap.max_keycode().raw();
                                let shift_keycode = all_keycodes
                                    .clone()
                                    .find(|i| {
                                        keymap
                                            .key_get_syms_by_level(xkb::Keycode::new(*i), 0, 0)
                                            .contains(&xkb::Keysym::Shift_L)
                                    })
                                    .unwrap();
                                let s = "Hello world!";
                                for c in s.chars() {
                                    let keysym = xkb::Keysym::from_char(c);
                                    let mut keycode = None;
                                    let mut shift = false;
                                    'outer: for i in all_keycodes.clone() {
                                        for j in 0..=1 {
                                            let syms = keymap.key_get_syms_by_level(
                                                xkb::Keycode::new(i),
                                                0,
                                                j,
                                            );
                                            if syms.contains(&keysym) {
                                                keycode = Some(i);
                                                shift = j == 1;
                                                break 'outer;
                                            }
                                        }
                                    }
                                    let keycode = keycode.unwrap();
                                    if shift {
                                        keyboard
                                            .key(shift_keycode - 8, ei::keyboard::KeyState::Press);
                                    }
                                    keyboard.key(keycode - 8, ei::keyboard::KeyState::Press);
                                    keyboard.key(keycode - 8, ei::keyboard::KeyState::Released);
                                    if shift {
                                        keyboard.key(
                                            shift_keycode - 8,
                                            ei::keyboard::KeyState::Released,
                                        );
                                    }
                                }
                                device.frame(self.last_serial, 1); // XXX time
                                device.stop_emulating(self.last_serial);
                                //self.running = false;
                            }
                        }
                        ei::device::Event::Resumed { serial } => {
                            self.last_serial = serial;
                        }
                        _ => {}
                    }
                }
                ei::Event::Keyboard(
                    _keyboard,
                    ei::keyboard::Event::Keymap {
                        keymap_type: _,
                        size,
                        keymap,
                    },
                ) => {
                    // XXX format
                    // flags?
                    // handle multiple keyboard?
                    let context = xkb::Context::new(0);
                    self.keymap = Some(
                        unsafe {
                            xkb::Keymap::new_from_fd(
                                &context,
                                keymap,
                                size as _,
                                xkb::KEYMAP_FORMAT_TEXT_V1,
                                0,
                            )
                        }
                        .unwrap()
                        .unwrap(),
                    );
                }
                _ => {}
            }
        }

        let _ = context.flush();

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
            .select_devices(
                &session,
                DeviceType::Keyboard.into(),
                None,
                PersistMode::DoNot,
            )
            .await
            .unwrap();
        remote_desktop.start(&session, None).await.unwrap();
        let fd = remote_desktop.connect_to_eis(&session).await.unwrap();
        let stream = UnixStream::from(fd);
        ei::Context::new(stream).unwrap()
    }
}

fn main() {
    let mut event_loop = calloop::EventLoop::try_new().unwrap();
    let handle = event_loop.handle();

    let context = futures_executor::block_on(open_connection());
    // XXX wait for server version?
    let _handshake = context.handshake();
    let _ = context.flush();
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
        last_serial: u32::MAX,
        sequence: 0,
        keymap: None,
    };
    while state.running {
        event_loop.dispatch(None, &mut state).unwrap();
    }
}
