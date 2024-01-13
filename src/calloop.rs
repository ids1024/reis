// TODO Define an event source that reads socket and produces eis::Event
// - is it easy to compose and wrap with handshaker, event handler?
// produce an event of some kind on disconnect/eof?

use calloop::{generic::Generic, Interest, Mode, PostAction, Readiness, Token, TokenFactory};
use std::{collections::HashMap, io};

use crate::{
    eis,
    request::{DeviceCapability, EisRequestConverter},
    PendingRequestResult,
};

pub struct EisListenerSource {
    source: Generic<eis::Listener>,
}

impl EisListenerSource {
    pub fn new(listener: eis::Listener) -> Self {
        Self {
            source: Generic::new(listener, Interest::READ, Mode::Level),
        }
    }
}

impl calloop::EventSource for EisListenerSource {
    type Event = eis::Context;
    type Metadata = ();
    type Ret = io::Result<PostAction>;
    type Error = io::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut cb: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut(eis::Context, &mut ()) -> io::Result<PostAction>,
    {
        self.source
            .process_events(readiness, token, |_readiness, listener| {
                if let Some(context) = listener.accept()? {
                    cb(context, &mut ())
                } else {
                    Ok(PostAction::Continue)
                }
            })
    }

    fn register(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut TokenFactory,
    ) -> Result<(), calloop::Error> {
        self.source.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut TokenFactory,
    ) -> Result<(), calloop::Error> {
        self.source.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> Result<(), calloop::Error> {
        self.source.unregister(poll)
    }
}

pub struct ConnectedContextState {
    context: eis::Context,
    connection: eis::Connection,
    name: Option<String>,
    context_type: eis::handshake::ContextType,
    seat: crate::request::Seat,
    negotiated_interfaces: HashMap<String, u32>,
    request_converter: crate::request::EisRequestConverter,
}

enum ContextState {
    Handshake(crate::handshake::EisHandshaker<'static>),
    Connected(ConnectedContextState),
}

pub struct EisRequestSource {
    source: Generic<eis::Context>,
    state: ContextState,
}

impl EisRequestSource {
    pub fn new(
        context: eis::Context,
        interfaces: &'static HashMap<&'static str, u32>,
        initial_serial: u32,
    ) -> Self {
        Self {
            state: ContextState::Handshake(crate::handshake::EisHandshaker::new(
                &context,
                interfaces,
                initial_serial,
            )),
            source: Generic::new(context, Interest::READ, Mode::Level),
        }
    }
}

impl calloop::EventSource for EisRequestSource {
    type Event = crate::request::EisRequest;
    type Metadata = ConnectedContextState;
    type Ret = io::Result<PostAction>;
    type Error = io::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut cb: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut(crate::request::EisRequest, &mut ConnectedContextState) -> io::Result<PostAction>,
    {
        self.source
            .process_events(readiness, token, |_readiness, context| {
                context.read()?;

                while let Some(result) = context.pending_request() {
                    let request = match result {
                        PendingRequestResult::Request(request) => request,
                        PendingRequestResult::ParseError(msg) => {
                            // TODO
                            return Ok(calloop::PostAction::Remove);
                        }
                        PendingRequestResult::InvalidObject(object_id) => {
                            // TODO
                            continue;
                        }
                    };

                    match &mut self.state {
                        ContextState::Handshake(handshaker) => {
                            match handshaker.handle_request(request) {
                                Ok(Some(resp)) => {
                                    // TODO Need on connect event

                                    if !resp.negotiated_interfaces.contains_key("ei_seat")
                                        || !resp.negotiated_interfaces.contains_key("ei_device")
                                    {
                                        // TODO
                                        /*
                                        resp.connection.disconnected(
                                            1,
                                            eis::connection::DisconnectReason::Protocol,
                                            "Need `ei_seat` and `ei_device`",
                                        );
                                        context.flush();
                                        */
                                        return Ok(calloop::PostAction::Remove);
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
                                    self.state = ContextState::Connected(connected_state);
                                }
                                Ok(None) => {}
                                Err(err) => {
                                    return Ok(calloop::PostAction::Remove);
                                }
                            }
                        }
                        ContextState::Connected(ref mut connected_state) => {
                            if let Err(err) =
                                connected_state.request_converter.handle_request(request)
                            {
                                // TODO
                                /*
                                return connected_state
                                    .protocol_error(&format!("request error: {:?}", err));
                                */
                            }
                            while let Some(request) =
                                connected_state.request_converter.next_request()
                            {
                                let res = cb(request, connected_state)?;
                                if res != calloop::PostAction::Continue {
                                    return Ok(res);
                                }
                            }
                        }
                    }
                }

                todo!()
            })
    }

    fn register(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut TokenFactory,
    ) -> Result<(), calloop::Error> {
        self.source.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut TokenFactory,
    ) -> Result<(), calloop::Error> {
        self.source.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> Result<(), calloop::Error> {
        self.source.unregister(poll)
    }
}
