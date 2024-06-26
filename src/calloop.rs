// TODO Define an event source that reads socket and produces eis::Event
// - is it easy to compose and wrap with handshaker, event handler?
// produce an event of some kind on disconnect/eof?

use calloop::{generic::Generic, Interest, Mode, PostAction, Readiness, Token, TokenFactory};
use std::{collections::HashMap, io};

use crate::{
    eis,
    handshake::HandshakeError,
    request::{self, EisRequestConverter},
    PendingRequestResult,
};

#[derive(Debug)]
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

#[derive(Debug)]
pub struct ConnectedContextState {
    pub context: eis::Context,
    pub connection: eis::Connection,
    pub name: Option<String>,
    pub context_type: eis::handshake::ContextType,
    pub negotiated_interfaces: HashMap<String, u32>,
    pub request_converter: request::EisRequestConverter,
}

impl ConnectedContextState {
    // Use type instead of string?
    pub fn has_interface(&self, interface: &str) -> bool {
        self.negotiated_interfaces.contains_key(interface)
    }

    fn process<F>(&mut self, mut cb: F) -> io::Result<PostAction>
    where
        F: FnMut(
            Result<EisRequestSourceEvent, request::Error>,
            &mut Self,
        ) -> io::Result<PostAction>,
    {
        if let Err(err) = self.context.read() {
            cb(Err(request::Error::Io(err)), self)?;
            return Ok(calloop::PostAction::Remove);
        }

        while let Some(result) = self.context.pending_request() {
            let request = match result {
                PendingRequestResult::Request(request) => request,
                PendingRequestResult::ParseError(err) => {
                    cb(Err(request::Error::Parse(err)), self)?;
                    return Ok(calloop::PostAction::Remove);
                }
                PendingRequestResult::InvalidObject(object_id) => {
                    let res = cb(Ok(EisRequestSourceEvent::InvalidObject(object_id)), self)?;
                    if res != calloop::PostAction::Continue {
                        return Ok(res);
                    }
                    continue;
                }
            };

            if let Err(err) = self.request_converter.handle_request(request) {
                cb(Err(err), self)?;
                return Ok(calloop::PostAction::Remove);
            }
            while let Some(request) = self.request_converter.next_request() {
                let res = cb(Ok(EisRequestSourceEvent::Request(request)), self)?;
                if res != calloop::PostAction::Continue {
                    return Ok(res);
                }
            }
        }

        Ok(calloop::PostAction::Continue)
    }
}

fn process_handshake(
    handshaker: &mut crate::handshake::EisHandshaker<'_>,
    context: &eis::Context,
) -> Result<Option<ConnectedContextState>, HandshakeError> {
    context.read()?;

    while let Some(result) = context.pending_request() {
        let request = crate::handshake::request_result(result)?;
        if let Some(resp) = handshaker.handle_request(request)? {
            let request_converter = EisRequestConverter::new(&resp.connection, 1);

            let connected_state = ConnectedContextState {
                context: context.clone(),
                connection: resp.connection,
                name: resp.name,
                context_type: resp.context_type,
                negotiated_interfaces: resp.negotiated_interfaces,
                request_converter,
            };

            return Ok(Some(connected_state));
        }
    }

    // XXX
    let _ = context.flush();

    Ok(None)
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum State {
    Handshake(crate::handshake::EisHandshaker<'static>),
    Connected(ConnectedContextState),
}

#[derive(Debug)]
pub struct EisRequestSource {
    source: Generic<eis::Context>,
    state: State,
}

impl EisRequestSource {
    pub fn new(
        context: eis::Context,
        interfaces: &'static HashMap<&'static str, u32>,
        initial_serial: u32,
    ) -> Self {
        let handshaker = crate::handshake::EisHandshaker::new(&context, interfaces, initial_serial);
        Self {
            source: Generic::new(context, Interest::READ, Mode::Level),
            state: State::Handshake(handshaker),
        }
    }
}

impl calloop::EventSource for EisRequestSource {
    type Event = Result<EisRequestSourceEvent, request::Error>;
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
        F: FnMut(Self::Event, &mut ConnectedContextState) -> io::Result<PostAction>,
    {
        self.source
            .process_events(readiness, token, |_readiness, context| {
                match &mut self.state {
                    State::Handshake(handshaker) => {
                        if let Some(res) = process_handshake(handshaker, context).transpose() {
                            match res {
                                Ok(mut state) => {
                                    let res = cb(Ok(EisRequestSourceEvent::Connected), &mut state)?;
                                    self.state = State::Connected(state);
                                    Ok(res)
                                }
                                Err(err) => {
                                    // TODO return handshake errors?
                                    eprintln!("Client handshake failed: {}", err);
                                    Ok(calloop::PostAction::Remove)
                                }
                            }
                        } else {
                            Ok(calloop::PostAction::Continue)
                        }
                    }
                    State::Connected(state) => state.process(&mut cb),
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

// TODO
pub enum EisRequestSourceEvent {
    Connected,
    Request(request::EisRequest),
    InvalidObject(u64),
}
