use calloop::generic::Generic;
use reis::eis;

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

                let source =
                    Generic::new(connection, calloop::Interest::READ, calloop::Mode::Level);
                state
                    .handle
                    .insert_source(source, |event, connection, state| {
                        let mut buf = [0; 16];
                        let mut fds = Vec::new();
                        let count = connection.recv(&mut buf, &mut fds).unwrap();
                        if count == 0 {
                            return Ok(calloop::PostAction::Remove);
                        }
                        assert_eq!(count, 16); // XXX bad
                        let header = reis::Header::parse(&buf).unwrap();

                        let mut buf = vec![0; header.length as usize - 16];
                        let mut fds = Vec::new();
                        let count = connection.recv(&mut buf, &mut fds).unwrap();
                        assert_eq!(count, buf.len());

                        if header.object_id == 0 {
                            let mut bytes = reis::ByteStream {
                                connection: &connection,
                                bytes: &buf,
                                fds: &mut fds,
                            };
                            let request =
                                eis::Request::parse("ei_handshake", header.opcode, &mut bytes);
                            println!("{:?}", request);
                        } else {
                            println!("Unknown {:?}", &header);
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
