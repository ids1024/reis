//! Typing text via the `ei_text` interface (libei 1.6.0).
//!
//! Unlike the `type-text` example, which reverse-maps each character to a
//! keycode through an xkb keymap and replays synthetic key presses, `ei_text`
//! takes a UTF-8 string directly. Run against a compositor that advertises the
//! `ei_text` capability.

use ashpd::desktop::{
    remote_desktop::{
        ConnectToEISOptions, DeviceType, RemoteDesktop, SelectDevicesOptions, StartOptions,
    },
    CreateSessionOptions, PersistMode,
};
use calloop::generic::Generic;
use enumflags2::BitFlags;
use once_cell::sync::Lazy;
use reis::{ei, PendingRequestResult};
use std::{collections::HashMap, io, os::unix::net::UnixStream};

static INTERFACES: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("ei_callback", 1);
    m.insert("ei_connection", 1);
    m.insert("ei_seat", 1);
    m.insert("ei_device", 1);
    m.insert("ei_pingpong", 1);
    m.insert("ei_text", 1);
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
}

impl State {
    #![allow(clippy::unnecessary_wraps)]
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
                        handshake.name("type-text-ei-example");
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
                            if let Some(mask) = data.capabilities.get("ei_text") {
                                seat.bind(*mask);
                            } else {
                                eprintln!(
                                    "Server does not advertise the ei_text capability \
                                     (requires libei 1.6.0 or newer)."
                                );
                            }
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
                            if let Some(text) = data.interface::<ei::Text>() {
                                // ei_text takes a UTF-8 string directly, so there is no
                                // need to reverse-map characters to keycodes through a keymap.
                                device.start_emulating(self.sequence, self.last_serial);
                                self.sequence += 1;
                                text.utf8("Hello world!");
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
        let session = remote_desktop
            .create_session(CreateSessionOptions::default())
            .await
            .unwrap();
        let options = SelectDevicesOptions::default()
            .set_devices(BitFlags::from(DeviceType::Keyboard))
            .set_persist_mode(PersistMode::DoNot);
        remote_desktop
            .select_devices(&session, options)
            .await
            .unwrap();
        remote_desktop
            .start(&session, None, StartOptions::default())
            .await
            .unwrap();
        let fd = remote_desktop
            .connect_to_eis(&session, ConnectToEISOptions::default())
            .await
            .unwrap();
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
    };
    while state.running {
        event_loop.dispatch(None, &mut state).unwrap();
    }
}
