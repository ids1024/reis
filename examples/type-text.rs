use calloop::generic::Generic;
use once_cell::sync::Lazy;
use reis::{ei, PendingRequestResult};
use std::{collections::HashMap, io};

static INTERFACES: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("ei_callback", 1);
    m.insert("ei_connection", 1);
    m.insert("ei_seat", 1);
    m.insert("ei_device", 1);
    m.insert("ei_pingpong", 1);
    m
});

struct State {}

impl State {
    fn handle_listener_readable(
        &mut self,
        context: &mut ei::Context,
    ) -> io::Result<calloop::PostAction> {
        match context.read() {
            Ok(res) if res.is_eof() => {
                return Ok(calloop::PostAction::Remove);
            }
            Err(_) => {
                return Ok(calloop::PostAction::Remove);
            }
            _ => {}
        }

        while let Some(result) = context.pending_event() {
            let request = match result {
                PendingRequestResult::Request(request) => request,
                PendingRequestResult::ProtocolError(msg) => {
                    todo!()
                }
                PendingRequestResult::InvalidObject(object_id) => {
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
                    _ => {}
                },
                _ => {}
            }
        }

        context.flush();

        Ok(calloop::PostAction::Continue)
    }
}

fn main() {
    let mut event_loop = calloop::EventLoop::try_new().unwrap();
    let handle = event_loop.handle();

    let context = ei::Context::connect_to_env().unwrap().unwrap();
    // XXX wait for server version?
    let handshake = context.handshake();
    context.flush();
    let context_source = Generic::new(context, calloop::Interest::READ, calloop::Mode::Level);
    handle
        .insert_source(context_source, |_event, context, state: &mut State| {
            state.handle_listener_readable(context)
        })
        .unwrap();

    let mut state = State {};
    event_loop.run(None, &mut state, |_| {}).unwrap();
}