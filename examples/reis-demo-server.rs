// TODO: Require context_type

use calloop::generic::Generic;
use once_cell::sync::Lazy;
use reis::{
    calloop::EisListenerSource,
    eis::{self, device::DeviceType},
    request::{DeviceCapability, EisRequest, EisRequestConverter},
    PendingRequestResult,
};
use std::{
    collections::HashMap,
    io,
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
    name: Option<String>,
    context_type: eis::handshake::ContextType,
    seat: reis::request::Seat,
    negotiated_interfaces: HashMap<String, u32>,
    request_converter: EisRequestConverter,
}

impl ConnectedContextState {
    fn disconnected(
        &self,
        reason: eis::connection::DisconnectReason,
        explaination: &str,
    ) -> calloop::PostAction {
        self.connection
            .disconnected(self.request_converter.last_serial(), reason, explaination);
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

    fn handle_request(&mut self, request: EisRequest) -> calloop::PostAction {
        match request {
            EisRequest::Disconnect => {
                return calloop::PostAction::Remove;
            }
            EisRequest::Bind(request) => {
                let capabilities = request.capabilities;

                // TODO Handle in converter
                if capabilities & 0x7e != capabilities {
                    let serial = self.request_converter.next_serial();
                    request.seat.eis_seat().destroyed(serial);
                    return self.disconnected(
                        eis::connection::DisconnectReason::Value,
                        "Invalid capabilities",
                    );
                }

                if self.has_interface("ei_keyboard")
                    && capabilities & 2 << DeviceCapability::Keyboard as u64 != 0
                {
                    self.request_converter.add_device(
                        &self.seat,
                        Some("keyboard"),
                        DeviceType::Virtual,
                        &[DeviceCapability::Keyboard],
                    );
                }

                // XXX button/etc should be on same object
                if self.has_interface("ei_pointer")
                    && capabilities & 2 << DeviceCapability::Pointer as u64 != 0
                {
                    self.request_converter.add_device(
                        &self.seat,
                        Some("pointer"),
                        DeviceType::Virtual,
                        &[DeviceCapability::Pointer],
                    );
                }

                if self.has_interface("ei_touchscreen")
                    && capabilities & 2 << DeviceCapability::Touch as u64 != 0
                {
                    self.request_converter.add_device(
                        &self.seat,
                        Some("touch"),
                        DeviceType::Virtual,
                        &[DeviceCapability::Touch],
                    );
                }

                if self.has_interface("ei_pointer_absolute")
                    && capabilities & 2 << DeviceCapability::PointerAbsolute as u64 != 0
                {
                    self.request_converter.add_device(
                        &self.seat,
                        Some("pointer-abs"),
                        DeviceType::Virtual,
                        &[DeviceCapability::PointerAbsolute],
                    );
                }

                // TODO create devices; compare against current bitflag
            }
            _ => {}
        }

        calloop::PostAction::Continue
    }
}

struct State {
    handle: calloop::LoopHandle<'static, Self>,
}

impl State {
    fn handle_new_connection(&mut self, context: eis::Context) -> io::Result<calloop::PostAction> {
        println!("New connection: {:?}", context);

        let handshaker = reis::handshake::EisHandshaker::new(&context, &SERVER_INTERFACES, 1);
        let mut context_state = ContextState::Handshake(context.clone(), handshaker);
        let source = Generic::new(context, calloop::Interest::READ, calloop::Mode::Level);
        self.handle
            .insert_source(source, move |_event, _context, state| {
                Ok(state.handle_connection_readable(&mut context_state))
            })
            .unwrap();

        Ok(calloop::PostAction::Continue)
    }

    fn handle_connection_readable(
        &mut self,
        context_state: &mut ContextState,
    ) -> calloop::PostAction {
        if let Err(_) = context_state.context().read() {
            return calloop::PostAction::Remove;
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
                        connected_state.connection.invalid_object(
                            connected_state.request_converter.last_serial(),
                            object_id,
                        );
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

                            let mut request_converter =
                                EisRequestConverter::new(&resp.connection, 1);
                            let seat = request_converter.add_seat(
                                Some("default"),
                                &[
                                    DeviceCapability::Pointer,
                                    DeviceCapability::PointerAbsolute,
                                    DeviceCapability::Keyboard,
                                    DeviceCapability::Touch,
                                    DeviceCapability::Scroll,
                                    DeviceCapability::Button,
                                ],
                            );

                            let connected_state = ConnectedContextState {
                                context: context.clone(),
                                connection: resp.connection,
                                name: resp.name,
                                context_type: resp.context_type,
                                seat,
                                negotiated_interfaces: resp.negotiated_interfaces.clone(),
                                request_converter,
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
                    if let Err(err) = connected_state.request_converter.handle_request(request) {
                        // TODO
                        return connected_state
                            .protocol_error(&format!("request error: {:?}", err));
                    }
                    while let Some(request) = connected_state.request_converter.next_request() {
                        let res = connected_state.handle_request(request);
                        if res != calloop::PostAction::Continue {
                            return res;
                        }
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
    let listener_source = EisListenerSource::new(listener);
    handle
        .insert_source(listener_source, |context, (), state: &mut State| {
            state.handle_new_connection(context)
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
