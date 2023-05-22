use calloop::generic::Generic;
use reis::{eis, PendingRequestResult};
use std::{
    io,
    os::unix::io::{AsRawFd, RawFd},
};

struct ConnectionState {
    connection: reis::Connection,
    handshake: eis::handshake::Handshake,
    connection_obj: Option<eis::connection::Connection>,
    last_serial: u32,
    name: Option<String>,
    context_type: Option<eis::handshake::ContextType>,
    seat: Option<eis::seat::Seat>,
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
    ) -> io::Result<calloop::PostAction> {
        if let Some(connection) = self.connection_obj.as_ref() {
            connection.disconnected(self.last_serial, reason, explaination);
        }
        Ok(calloop::PostAction::Remove)
    }

    fn protocol_error(&self, explanation: &str) -> io::Result<calloop::PostAction> {
        self.disconnected(eis::connection::DisconnectReason::Protocol, explanation)
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

            let handshake = connection.eis_handshake();
            handshake.handshake_version(1);
            handshake.interface_version("ei_callback", 1);
            handshake.interface_version("ei_connection", 1);
            handshake.interface_version("ei_seat", 1);

            let connection_state = ConnectionState {
                connection,
                handshake,
                connection_obj: None,
                last_serial: 0,
                name: None,
                context_type: None,
                seat: None,
            };
            let source = Generic::new(
                connection_state,
                calloop::Interest::READ,
                calloop::Mode::Level,
            );
            self.handle
                .insert_source(source, |_event, connection_state, state| {
                    state.handle_connection_readable(connection_state)
                })
                .unwrap();
        }

        Ok(calloop::PostAction::Continue)
    }

    fn handle_connection_readable(
        &mut self,
        connection_state: &mut ConnectionState,
    ) -> io::Result<calloop::PostAction> {
        if connection_state.connection.read()?.is_eof() {
            return Ok(calloop::PostAction::Remove);
        }

        while let Some(result) = connection_state.connection.eis_pending_request() {
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
                    eis::handshake::Request::InterfaceVersion { name, version } => {}
                    eis::handshake::Request::Finish => {
                        // May prompt user here whether to allow this
                        let serial = connection_state.next_serial();
                        let connection_obj =
                            connection_state.handshake.connection(serial, 1).unwrap();
                        // XXX only if protocol version supported by client
                        let seat = connection_obj.seat(1).unwrap();
                        seat.name("default");
                        seat.capability(0x1, "ei_pointer");
                        seat.capability(0x2, "ei_pointer_absolute");
                        seat.capability(0x4, "ei_button");
                        seat.capability(0x8, "ei_scroll");
                        seat.capability(0x10, "ei_keyboard");
                        seat.capability(0x20, "ei_touchscreen");
                        seat.done();
                        connection_state.seat = Some(seat);
                        connection_state.connection_obj = Some(connection_obj);
                    }
                    _ => {}
                },
                eis::Request::Connection(request) => match request {
                    eis::connection::Request::Disconnect => {
                        // Do not send `disconnected` in response
                        return Ok(calloop::PostAction::Remove);
                    }
                    eis::connection::Request::Sync { callback } => {
                        callback.done(0);
                    }
                    _ => {}
                },
                eis::Request::Seat(request) => match request {
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

        Ok(calloop::PostAction::Continue)
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
