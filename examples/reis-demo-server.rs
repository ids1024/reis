use calloop::generic::Generic;
use reis::eis;
use std::{collections::VecDeque, os::unix::io::{AsRawFd, OwnedFd, RawFd}};

struct ConnectionState {
    connection: reis::Connection,
    read_buffer: Vec<u8>,
    read_fds: Vec<OwnedFd>,
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

                let connection_state = ConnectionState {
                    connection,
                    read_buffer: Vec::new(),
                    read_fds: Vec::new(),
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
                        if count == 0 {
                            // TODO handle any messages first
                            return Ok(calloop::PostAction::Remove);
                        }
                        connection_state.read_buffer.extend_from_slice(&buf[0..count]);


                        if connection_state.read_buffer.len() < 16 {
                            return Ok(calloop::PostAction::Continue);
                        }

                        let header = reis::Header::parse(&buf).unwrap();
                        if connection_state.read_buffer.len() < header.length as usize {
                            return Ok(calloop::PostAction::Continue);
                        }

                        // XXX protocol error
                        if header.length < 16 {
                            return Ok(calloop::PostAction::Remove);
                        }

                        if header.object_id == 0 {
                            let mut bytes = reis::ByteStream {
                                connection: &connection_state.connection,
                                bytes: &connection_state.read_buffer[16..header.length as usize],
                                fds: &mut connection_state.read_fds,
                            };
                            let request =
                                eis::Request::parse("ei_handshake", header.opcode, &mut bytes);
                            println!("{:?}", request);
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
