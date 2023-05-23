use calloop::generic::Generic;
use once_cell::sync::Lazy;
use reis::{eis, PendingRequestResult};
use std::{
    collections::HashMap,
    io,
    os::unix::io::{AsRawFd, RawFd},
};

// TODO oncecell interfaces

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

struct ConnectionState {
    connection: eis::Connection,
    handshake: eis::handshake::Handshake,
    connection_obj: Option<eis::connection::Connection>,
    last_serial: u32,
    name: Option<String>,
    context_type: Option<eis::handshake::ContextType>,
    seat: Option<eis::seat::Seat>,
    negotiated_interfaces: HashMap<&'static str, u32>,
}

impl ConnectionState {
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

impl AsRawFd for ConnectionState {
    fn as_raw_fd(&self) -> RawFd {
        self.connection.as_raw_fd()
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
        while let Some(connection) = listener.accept()? {
            println!("New connection: {:?}", connection);

            let handshake = connection.handshake();
            handshake.handshake_version(1);
            for (interface, version) in SERVER_INTERFACES.iter() {
                handshake.interface_version(interface, *version);
            }

            let connection_state = ConnectionState {
                connection,
                handshake,
                connection_obj: None,
                last_serial: 0,
                name: None,
                context_type: None,
                seat: None,
                negotiated_interfaces: HashMap::new(),
            };
            let source = Generic::new(
                connection_state,
                calloop::Interest::READ,
                calloop::Mode::Level,
            );
            self.handle
                .insert_source(source, |_event, connection_state, state| {
                    Ok(state.handle_connection_readable(connection_state))
                })
                .unwrap();
        }

        Ok(calloop::PostAction::Continue)
    }

    fn handle_connection_readable(
        &mut self,
        connection_state: &mut ConnectionState,
    ) -> calloop::PostAction {
        match connection_state.connection.read() {
            Ok(res) if res.is_eof() => {
                return calloop::PostAction::Remove;
            }
            Err(_) => {
                return calloop::PostAction::Remove;
            }
            _ => {}
        }

        while let Some(result) = connection_state.connection.pending_request() {
            let request = match result {
                PendingRequestResult::Request(request) => request,
                PendingRequestResult::ProtocolError(msg) => {
                    return connection_state.protocol_error(msg);
                }
                PendingRequestResult::InvalidObject(object_id) => {
                    if let Some(connection) = connection_state.connection_obj.as_ref() {
                        // Only send if object ID is in range?
                        connection.invalid_object(connection_state.last_serial, object_id);
                    }
                    continue;
                }
            };
            println!("{:?}", request);
            match request {
                eis::Request::Handshake(request) => match request {
                    eis::handshake::Request::ContextType { context_type } => {
                        if connection_state.context_type.is_some() {
                            return connection_state
                                .protocol_error("context_type can only be sent once");
                        }
                        connection_state.context_type = Some(context_type);
                    }
                    eis::handshake::Request::Name { name } => {
                        if connection_state.name.is_some() {
                            return connection_state.protocol_error("name can only be sent once");
                        }
                        connection_state.name = Some(name);
                    }
                    eis::handshake::Request::InterfaceVersion { name, version } => {
                        if let Some((interface, server_version)) =
                            SERVER_INTERFACES.get_key_value(name.as_str())
                        {
                            connection_state
                                .negotiated_interfaces
                                .insert(interface, version.min(*server_version));
                        }
                    }
                    eis::handshake::Request::Finish => {
                        // May prompt user here whether to allow this

                        if !connection_state.has_interface("ei_connection")
                            || !connection_state.has_interface("ei_pingpong")
                            || !connection_state.has_interface("ei_callback")
                        {
                            return calloop::PostAction::Remove;
                        }

                        let serial = connection_state.next_serial();
                        let connection_obj =
                            connection_state.handshake.connection(serial, 1).unwrap();
                        connection_state.connection_obj = Some(connection_obj.clone());
                        if !connection_state.has_interface("ei_seat")
                            || !connection_state.has_interface("ei_device")
                            || !connection_state.has_interface("ei_callback")
                        {
                            return connection_state
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
                        connection_state.seat = Some(seat);
                        connection_state.connection_obj = Some(connection_obj);
                    }
                    _ => {}
                },
                eis::Request::Connection(request) => match request {
                    eis::connection::Request::Disconnect => {
                        // Do not send `disconnected` in response
                        return calloop::PostAction::Remove;
                    }
                    eis::connection::Request::Sync { callback } => {
                        callback.done(0);
                    }
                    _ => {}
                },
                eis::Request::Seat(request) => match request {
                    eis::seat::Request::Bind { capabilities } => {
                        if capabilities & 0x7e != capabilities {
                            let serial = connection_state.next_serial();
                            let seat = connection_state.seat.as_ref().unwrap(); // XXX
                            seat.destroyed(serial);
                            return connection_state.disconnected(
                                eis::connection::DisconnectReason::Value,
                                "Invalid capabilities",
                            );
                        }
                        let seat = connection_state.seat.as_ref().unwrap(); // XXX
                        let device = seat.device(1).unwrap();
                        device.name("keyboard");
                        device.device_type(eis::device::DeviceType::Virtual);
                        // XXX how to indicate type as return
                        // - first argument could be populated using a type generic
                        device.interface("ei_keyboard", 1);
                        device.done();
                        // TODO create devices; compare against current bitflag
                    }
                    eis::seat::Request::Release => {
                        // XXX
                        let serial = connection_state.next_serial();
                        connection_state.seat.as_ref().unwrap().destroyed(serial);
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
