// TODO: Require context_type

use calloop::generic::Generic;
use once_cell::sync::Lazy;
use reis::{eis, PendingRequestResult};
use std::{
    collections::HashMap,
    io,
    os::unix::io::{AsFd, BorrowedFd},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

static SERVER_INTERFACES: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("ei_callback", 1);
    m.insert("ei_connection", 1);
    m.insert("ei_seat", 1);
    m.insert("ei_device", 1);
    m.insert("ei_pingpong", 1);
    m.insert("ei_keyboard", 1);
    m.insert("ei_pointer", 1);
    m.insert("ei_pointer_absolute", 1);
    m.insert("ei_button", 1);
    m.insert("ei_scroll", 1);
    m.insert("ei_touchscreen", 1);
    m
});

enum ContextState {
    Handshake(eis::Context, reis::handshake::EisHandshaker<'static>),
    Connected(ConnectedContextState),
}

impl ContextState {
    fn context(&self) -> &eis::Context {
        match self {
            ContextState::Handshake(context, _) => context,
            ContextState::Connected(state) => &state.context,
        }
    }
}

struct ConnectedContextState {
    context: eis::Context,
    connection: eis::Connection,
    last_serial: u32,
    name: Option<String>,
    context_type: eis::handshake::ContextType,
    seat: eis::Seat,
    negotiated_interfaces: HashMap<String, u32>,
}

impl ConnectedContextState {
    fn next_serial(&mut self) -> u32 {
        self.last_serial += 1;
        self.last_serial
    }

    fn disconnected(
        &self,
        reason: eis::connection::DisconnectReason,
        explaination: &str,
    ) -> calloop::PostAction {
        self.connection
            .disconnected(self.last_serial, reason, explaination);
        self.context.flush();
        calloop::PostAction::Remove
    }

    fn protocol_error(&self, explanation: &str) -> calloop::PostAction {
        self.disconnected(eis::connection::DisconnectReason::Protocol, explanation)
    }

    // Use type instead of string?
    fn has_interface(&self, interface: &str) -> bool {
        self.negotiated_interfaces.contains_key(interface)
    }

    fn handle_request(&mut self, request: eis::Request) -> calloop::PostAction {
        match request {
            eis::Request::Handshake(handshake, request) => {}
            eis::Request::Connection(_connection, request) => match request {
                eis::connection::Request::Disconnect => {
                    // Do not send `disconnected` in response
                    return calloop::PostAction::Remove;
                }
                eis::connection::Request::Sync { callback } => {
                    if callback.version() != 1 {
                        return self.protocol_error("Invalid protocol object version");
                    }
                    callback.done(0);
                }
                _ => {}
            },
            eis::Request::Seat(seat, request) => match request {
                eis::seat::Request::Bind { capabilities } => {
                    if capabilities & 0x7e != capabilities {
                        let serial = self.next_serial();
                        seat.destroyed(serial);
                        return self.disconnected(
                            eis::connection::DisconnectReason::Value,
                            "Invalid capabilities",
                        );
                    }

                    fn add_device<T: eis::Interface>(seat: &eis::Seat, name: &str) {
                        let device = seat.device(1);
                        device.name(name);
                        device.device_type(eis::device::DeviceType::Virtual);
                        device.interface::<T>(1);
                        device.done();
                    }

                    if self.has_interface("ei_keyboard") && capabilities & 0x20 != 0 {
                        add_device::<eis::Keyboard>(&seat, "keyboard");
                    }

                    // XXX button/etc should be on same object
                    if self.has_interface("ei_pointer") && capabilities & 0x2 != 0 {
                        add_device::<eis::Pointer>(&seat, "pointer");
                    }

                    if self.has_interface("ei_touchscreen") && capabilities & 0x40 != 0 {
                        add_device::<eis::Touchscreen>(&seat, "touch");
                    }

                    if self.has_interface("ei_pointer_absolute") && capabilities & 0x4 != 0 {
                        add_device::<eis::PointerAbsolute>(&seat, "pointer-abs");
                    }

                    // TODO create devices; compare against current bitflag
                }
                eis::seat::Request::Release => {
                    // XXX
                    let serial = self.next_serial();
                    seat.destroyed(serial);
                }
                _ => {}
            },
            _ => {}
        }

        calloop::PostAction::Continue
    }
}

impl AsFd for ContextState {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.context().as_fd()
    }
}

struct State {
    handle: calloop::LoopHandle<'static, Self>,
}

impl State {
    fn handle_listener_readable(
        &mut self,
        listener: &mut eis::Listener,
    ) -> io::Result<calloop::PostAction> {
        while let Some(context) = listener.accept()? {
            println!("New connection: {:?}", context);

            let handshaker = reis::handshake::EisHandshaker::new(&context, &SERVER_INTERFACES, 1);
            let context_state = ContextState::Handshake(context, handshaker);
            let source = Generic::new(context_state, calloop::Interest::READ, calloop::Mode::Level);
            self.handle
                .insert_source(source, |_event, context_state, state| {
                    // XXX How can calloop avoid unsafe here?
                    Ok(state.handle_connection_readable(unsafe { context_state.get_mut() }))
                })
                .unwrap();
        }

        Ok(calloop::PostAction::Continue)
    }

    fn handle_connection_readable(
        &mut self,
        context_state: &mut ContextState,
    ) -> calloop::PostAction {
        match context_state.context().read() {
            Ok(res) if res.is_eof() => {
                return calloop::PostAction::Remove;
            }
            Err(_) => {
                return calloop::PostAction::Remove;
            }
            _ => {}
        }

        while let Some(result) = context_state.context().pending_request() {
            let request = match result {
                PendingRequestResult::Request(request) => request,
                PendingRequestResult::ParseError(msg) => {
                    if let ContextState::Connected(connected_state) = context_state {
                        return connected_state.protocol_error(&format!("parse error: {}", msg));
                    }
                    return calloop::PostAction::Remove;
                }
                PendingRequestResult::InvalidObject(object_id) => {
                    if let ContextState::Connected(connected_state) = context_state {
                        // Only send if object ID is in range?
                        connected_state
                            .connection
                            .invalid_object(connected_state.last_serial, object_id);
                    }
                    continue;
                }
            };

            match context_state {
                ContextState::Handshake(context, handshaker) => {
                    match handshaker.handle_request(request) {
                        Ok(Some(resp)) => {
                            if !resp.negotiated_interfaces.contains_key("ei_seat")
                                || !resp.negotiated_interfaces.contains_key("ei_device")
                            {
                                resp.connection.disconnected(
                                    1,
                                    eis::connection::DisconnectReason::Protocol,
                                    "Need `ei_seat` and `ei_device`",
                                );
                                context.flush();
                                return calloop::PostAction::Remove;
                            }

                            let seat = resp.connection.seat(1);
                            seat.name("default");
                            seat.capability(0x2, "ei_pointer");
                            seat.capability(0x4, "ei_pointer_absolute");
                            seat.capability(0x8, "ei_button");
                            seat.capability(0x10, "ei_scroll");
                            seat.capability(0x20, "ei_keyboard");
                            seat.capability(0x40, "ei_touchscreen");
                            seat.done();

                            let connected_state = ConnectedContextState {
                                context: context.clone(),
                                connection: resp.connection,
                                last_serial: 1,
                                name: resp.name,
                                context_type: resp.context_type,
                                seat: seat,
                                negotiated_interfaces: resp.negotiated_interfaces.clone(),
                            };
                            *context_state = ContextState::Connected(connected_state);
                        }
                        Ok(None) => {}
                        Err(err) => {
                            return calloop::PostAction::Remove;
                        }
                    }
                }
                ContextState::Connected(connected_state) => {
                    let res = connected_state.handle_request(request);
                    if res != calloop::PostAction::Continue {
                        return res;
                    }
                }
            }
        }

        // XXX handle error and WouldBlock
        context_state.context().flush();

        calloop::PostAction::Continue
    }
}

fn main() {
    let mut event_loop = calloop::EventLoop::try_new().unwrap();
    let handle = event_loop.handle();

    let path = reis::default_socket_path().unwrap();
    std::fs::remove_file(&path); // XXX in use?
    let listener = eis::Listener::bind(&path).unwrap();
    let listener_source = Generic::new(listener, calloop::Interest::READ, calloop::Mode::Level);
    handle
        .insert_source(listener_source, |_event, listener, state: &mut State| {
            state.handle_listener_readable(unsafe { listener.get_mut() })
        })
        .unwrap();

    let terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, terminate.clone()).unwrap();

    let mut state = State { handle };
    while !terminate.load(Ordering::Relaxed) {
        event_loop
            .dispatch(Duration::from_millis(100), &mut state)
            .unwrap();
    }
}
