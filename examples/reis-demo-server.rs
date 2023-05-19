use calloop::generic::Generic;
use reis::eis;
use std::{
    collections::VecDeque,
    io,
    os::unix::io::{AsRawFd, OwnedFd, RawFd},
};

struct ConnectionState {
    connection: reis::Connection,
    read_buffer: Vec<u8>,
    read_fds: Vec<OwnedFd>,
    handshake: eis::handshake::Handshake,
    connection_obj: Option<eis::connection::Connection>,
    last_serial: u32,
    name: Option<String>,
    context_type: Option<eis::handshake::ContextType>,
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

fn main() {
    let mut event_loop = calloop::EventLoop::try_new().unwrap();
    let handle = event_loop.handle();

    let path = reis::default_socket_path().unwrap();
    std::fs::remove_file(&path); // XXX in use?
    let listener = eis::Listener::bind(&path).unwrap();
    let listener_source = Generic::new(listener, calloop::Interest::READ, calloop::Mode::Level);
    handle
        .insert_source(listener_source, |event, listener, state: &mut State| {
            while let Some(connection) = listener.accept()? {
                println!("New connection: {:?}", connection);

                let handshake = connection.eis_handshake();
                handshake.handshake_version(1);
                handshake.interface_version("ei_callback", 1);
                handshake.interface_version("ei_connection", 1);
                handshake.interface_version("ei_seat", 1);

                let connection_state = ConnectionState {
                    connection,
                    read_buffer: Vec::new(),
                    read_fds: Vec::new(),
                    handshake,
                    connection_obj: None,
                    last_serial: 0,
                    name: None,
                    context_type: None,
                };
                let source = Generic::new(
                    connection_state,
                    calloop::Interest::READ,
                    calloop::Mode::Level,
                );
                state
                    .handle
                    .insert_source(source, |event, connection_state, state| {
                        let mut buf = [0; 2048];
                        let count = connection_state
                            .connection
                            .recv(&mut buf, &mut connection_state.read_fds)
                            .unwrap();
                        if count == 0 && connection_state.read_buffer.len() < 16 {
                            return Ok(calloop::PostAction::Remove);
                        }
                        connection_state
                            .read_buffer
                            .extend_from_slice(&buf[0..count]);

                        if connection_state.read_buffer.len() < 16 {
                            return Ok(calloop::PostAction::Continue);
                        }

                        let header = reis::Header::parse(&buf).unwrap();
                        if connection_state.read_buffer.len() < header.length as usize {
                            return Ok(calloop::PostAction::Continue);
                        }

                        if header.length < 16 {
                            return connection_state.protocol_error("header length < 16");
                        }

                        if let Some(interface) = connection_state
                            .connection
                            .object_interface(header.object_id)
                        {
                            let mut bytes = reis::ByteStream {
                                connection: &connection_state.connection,
                                bytes: &connection_state.read_buffer[16..header.length as usize],
                                fds: &mut connection_state.read_fds,
                            };
                            let request = eis::Request::parse(interface, header.opcode, &mut bytes);
                            println!("{:?}", request);
                            match request {
                                Some(eis::Request::Handshake(request)) => match request {
                                    eis::handshake::Request::ContextType { context_type } => {
                                        if connection_state.context_type.is_some() {
                                            return connection_state.protocol_error(
                                                "context_type can only be sent once",
                                            );
                                        }
                                        connection_state.context_type = Some(context_type);
                                    }
                                    eis::handshake::Request::Name { name } => {
                                        if connection_state.name.is_some() {
                                            return connection_state
                                                .protocol_error("name can only be sent once");
                                        }
                                        connection_state.name = Some(name);
                                    }
                                    eis::handshake::Request::Finish => {
                                        // May prompt user here whether to allow this
                                        let serial = connection_state.next_serial();
                                        connection_state.connection_obj = Some(
                                            connection_state
                                                .handshake
                                                .connection(serial, 1)
                                                .unwrap(),
                                        );
                                    }
                                    _ => {}
                                },
                                Some(eis::Request::Connection(request)) => match request {
                                    eis::connection::Request::Disconnect => {
                                        // Do not send `disconnected` in response
                                        return Ok(calloop::PostAction::Remove);
                                    }
                                    eis::connection::Request::Sync { callback } => {
                                        callback.done(0);
                                    }
                                    _ => {}
                                },
                                None => {
                                    return connection_state
                                        .protocol_error("failed to parse request");
                                }
                                _ => {}
                            }
                        } else {
                            println!("Unknown {:?}", &header);
                        }

                        // XXX inefficient
                        for i in 0..header.length as usize {
                            connection_state.read_buffer.remove(0);
                        }

                        Ok(calloop::PostAction::Continue)
                    })
                    .unwrap();
            }
            Ok(calloop::PostAction::Continue)
        })
        .unwrap();

    let mut state = State { handle };
    event_loop.run(None, &mut state, |_| {}).unwrap();
}
