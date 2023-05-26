use calloop::generic::Generic;
use once_cell::sync::Lazy;
use reis::{eis, PendingRequestResult};
use std::{
    collections::HashMap,
    io,
    os::unix::io::{AsRawFd, RawFd},
};

static SERVER_INTERFACES: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("ei_callback", 1);
    m.insert("ei_connection", 1);
    m.insert("ei_seat", 1);
    m.insert("ei_device", 1);
    m.insert("ei_callback", 1);
    m.insert("ei_pingpong", 1);
    m
});

struct ContextState {
    context: eis::Context,
    handshake: eis::Handshake,
    connection_obj: Option<eis::Connection>,
    last_serial: u32,
    name: Option<String>,
    context_type: Option<eis::handshake::ContextType>,
    seat: Option<eis::Seat>,
    negotiated_interfaces: HashMap<&'static str, u32>,
}

impl ContextState {
    fn next_serial(&mut self) -> u32 {
        self.last_serial += 1;
        self.last_serial
    }

    fn disconnected(
        &self,
        reason: eis::connection::DisconnectReason,
        explaination: &str,
    ) -> calloop::PostAction {
        if let Some(connection) = self.connection_obj.as_ref() {
            connection.disconnected(self.last_serial, reason, explaination);
        }
        calloop::PostAction::Remove
    }

    fn protocol_error(&self, explanation: &str) -> calloop::PostAction {
        self.disconnected(eis::connection::DisconnectReason::Protocol, explanation)
    }

    // Use type instead of string?
    fn has_interface(&self, interface: &str) -> bool {
        self.negotiated_interfaces.contains_key(interface)
    }
}

impl AsRawFd for ContextState {
    fn as_raw_fd(&self) -> RawFd {
        self.context.as_raw_fd()
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

            let handshake = context.handshake();
            handshake.handshake_version(1);
            for (interface, version) in SERVER_INTERFACES.iter() {
                handshake.interface_version(interface, *version);
            }

            let context_state = ContextState {
                context,
                handshake,
                connection_obj: None,
                last_serial: 0,
                name: None,
                context_type: None,
                seat: None,
                negotiated_interfaces: HashMap::new(),
            };
            let source = Generic::new(context_state, calloop::Interest::READ, calloop::Mode::Level);
            self.handle
                .insert_source(source, |_event, context_state, state| {
                    Ok(state.handle_connection_readable(context_state))
                })
                .unwrap();
        }

        Ok(calloop::PostAction::Continue)
    }

    fn handle_connection_readable(
        &mut self,
        context_state: &mut ContextState,
    ) -> calloop::PostAction {
        match context_state.context.read() {
            Ok(res) if res.is_eof() => {
                return calloop::PostAction::Remove;
            }
            Err(_) => {
                return calloop::PostAction::Remove;
            }
            _ => {}
        }

        while let Some(result) = context_state.context.pending_request() {
            let request = match result {
                PendingRequestResult::Request(request) => request,
                PendingRequestResult::ProtocolError(msg) => {
                    return context_state.protocol_error(msg);
                }
                PendingRequestResult::InvalidObject(object_id) => {
                    if let Some(connection) = context_state.connection_obj.as_ref() {
                        // Only send if object ID is in range?
                        connection.invalid_object(context_state.last_serial, object_id);
                    }
                    continue;
                }
            };
            println!("{:?}", request);
            match request {
                eis::Request::Handshake(_handshake, request) => match request {
                    eis::handshake::Request::ContextType { context_type } => {
                        if context_state.context_type.is_some() {
                            return context_state
                                .protocol_error("context_type can only be sent once");
                        }
                        context_state.context_type = Some(context_type);
                    }
                    eis::handshake::Request::Name { name } => {
                        if context_state.name.is_some() {
                            return context_state.protocol_error("name can only be sent once");
                        }
                        context_state.name = Some(name);
                    }
                    eis::handshake::Request::InterfaceVersion { name, version } => {
                        if let Some((interface, server_version)) =
                            SERVER_INTERFACES.get_key_value(name.as_str())
                        {
                            context_state
                                .negotiated_interfaces
                                .insert(interface, version.min(*server_version));
                        }
                    }
                    eis::handshake::Request::Finish => {
                        // May prompt user here whether to allow this

                        if !context_state.has_interface("ei_connection")
                            || !context_state.has_interface("ei_pingpong")
                            || !context_state.has_interface("ei_callback")
                        {
                            return calloop::PostAction::Remove;
                        }

                        let serial = context_state.next_serial();
                        let connection_obj = context_state.handshake.connection(serial, 1).unwrap();
                        context_state.connection_obj = Some(connection_obj.clone());
                        if !context_state.has_interface("ei_seat")
                            || !context_state.has_interface("ei_device")
                            || !context_state.has_interface("ei_callback")
                        {
                            return context_state
                                .disconnected(eis::connection::DisconnectReason::Disconnected, "");
                            // XXX reason
                        }
                        let seat = connection_obj.seat(1).unwrap();
                        seat.name("default");
                        seat.capability(0x2, "ei_pointer");
                        seat.capability(0x4, "ei_pointer_absolute");
                        seat.capability(0x8, "ei_button");
                        seat.capability(0x10, "ei_scroll");
                        seat.capability(0x20, "ei_keyboard");
                        seat.capability(0x40, "ei_touchscreen");
                        seat.done();
                        context_state.seat = Some(seat);
                        context_state.connection_obj = Some(connection_obj);
                    }
                    _ => {}
                },
                eis::Request::Connection(_connection, request) => match request {
                    eis::connection::Request::Disconnect => {
                        // Do not send `disconnected` in response
                        return calloop::PostAction::Remove;
                    }
                    eis::connection::Request::Sync { callback } => {
                        callback.done(0);
                    }
                    _ => {}
                },
                eis::Request::Seat(seat, request) => match request {
                    eis::seat::Request::Bind { capabilities } => {
                        if capabilities & 0x7e != capabilities {
                            let serial = context_state.next_serial();
                            seat.destroyed(serial);
                            return context_state.disconnected(
                                eis::connection::DisconnectReason::Value,
                                "Invalid capabilities",
                            );
                        }
                        let device = seat.device(1).unwrap();
                        device.name("keyboard");
                        device.device_type(eis::device::DeviceType::Virtual);
                        device.interface::<eis::Keyboard>(1);
                        device.done();
                        // TODO create devices; compare against current bitflag
                    }
                    eis::seat::Request::Release => {
                        // XXX
                        let serial = context_state.next_serial();
                        seat.destroyed(serial);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

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
            state.handle_listener_readable(listener)
        })
        .unwrap();

    let mut state = State { handle };
    event_loop.run(None, &mut state, |_| {}).unwrap();
}
