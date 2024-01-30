// TODO Define an event source that reads socket and produces eis::Event
// - is it easy to compose and wrap with handshaker, event handler?
// produce an event of some kind on disconnect/eof?

use calloop::{generic::Generic, Interest, Mode, PostAction, Readiness, Token, TokenFactory};
use std::{collections::HashMap, io};

use crate::{
    eis,
    handshake::{EisHandshakeResp, HandshakeError},
    request::EisRequestConverter,
    ParseError, PendingRequestResult,
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
    pub context: eis::Context,
    pub connection: eis::Connection,
    pub name: Option<String>,
    pub context_type: eis::handshake::ContextType,
    pub negotiated_interfaces: HashMap<String, u32>,
    pub request_converter: crate::request::EisRequestConverter,
}

impl ConnectedContextState {
    // Use type instead of string?
    pub fn has_interface(&self, interface: &str) -> bool {
        self.negotiated_interfaces.contains_key(interface)
    }
}

pub struct EisHandshakeSource {
    handshaker: crate::handshake::EisHandshaker<'static>,
    source: Generic<eis::Context>,
}

impl EisHandshakeSource {
    pub fn new(
        context: eis::Context,
        interfaces: &'static HashMap<&'static str, u32>,
        initial_serial: u32,
    ) -> Self {
        Self {
            handshaker: crate::handshake::EisHandshaker::new(&context, interfaces, initial_serial),
            source: Generic::new(context, Interest::READ, Mode::Level),
        }
    }
}

impl calloop::EventSource for EisHandshakeSource {
    type Event = Result<EisHandshakeResp, HandshakeError>;
    type Metadata = ();
    type Ret = io::Result<()>;
    type Error = io::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut cb: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut(Result<EisHandshakeResp, HandshakeError>, &mut ()) -> io::Result<()>,
    {
        self.source
            .process_events(readiness, token, |_readiness, context| {
                if let Err(err) = context.read() {
                    cb(Err(HandshakeError::Io(err)), &mut ())?;
                    return Ok(calloop::PostAction::Remove);
                }

                while let Some(result) = context.pending_request() {
                    let request = match result {
                        PendingRequestResult::Request(request) => request,
                        PendingRequestResult::ParseError(err) => {
                            cb(Err(HandshakeError::Parse(err)), &mut ())?;
                            return Ok(calloop::PostAction::Remove);
                        }
                        PendingRequestResult::InvalidObject(object_id) => {
                            cb(Err(HandshakeError::InvalidObject(object_id)), &mut ())?;
                            return Ok(calloop::PostAction::Remove);
                        }
                    };
                    match self.handshaker.handle_request(request) {
                        Ok(Some(resp)) => {
                            cb(Ok(resp), &mut ())?;
                            return Ok(calloop::PostAction::Remove);
                        }
                        Ok(None) => {}
                        Err(err) => {
                            cb(Err(err), &mut ())?;
                            return Ok(calloop::PostAction::Remove);
                        }
                    }
                    // XXX
                    let _ = context.flush();
                }

                Ok(calloop::PostAction::Continue)
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

pub struct EisRequestSource {
    source: Generic<eis::Context>,
    pub state: ConnectedContextState,
}

impl EisRequestSource {
    pub fn new(context: eis::Context, resp: EisHandshakeResp) -> Self {
        let request_converter = EisRequestConverter::new(&resp.connection, 1);

        let connected_state = ConnectedContextState {
            context: context.clone(),
            connection: resp.connection,
            name: resp.name,
            context_type: resp.context_type,
            negotiated_interfaces: resp.negotiated_interfaces,
            request_converter,
        };

        Self {
            source: Generic::new(context, Interest::READ, Mode::Level),
            state: connected_state,
        }
    }
}

impl calloop::EventSource for EisRequestSource {
    // type Event = crate::request::EisRequest;
    type Event = EisRequestSourceEvent;
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
        F: FnMut(EisRequestSourceEvent, &mut ConnectedContextState) -> io::Result<PostAction>,
    {
        self.source
            .process_events(readiness, token, |_readiness, context| {
                if let Err(err) = context.read() {
                    cb(EisRequestSourceEvent::IoError(err), &mut self.state)?;
                    return Ok(calloop::PostAction::Remove);
                }

                while let Some(result) = context.pending_request() {
                    let request = match result {
                        PendingRequestResult::Request(request) => request,
                        PendingRequestResult::ParseError(err) => {
                            cb(EisRequestSourceEvent::ParseError(err), &mut self.state)?;
                            return Ok(calloop::PostAction::Remove);
                        }
                        PendingRequestResult::InvalidObject(object_id) => {
                            let res = cb(
                                EisRequestSourceEvent::InvalidObject(object_id),
                                &mut self.state,
                            )?;
                            if res != calloop::PostAction::Continue {
                                return Ok(res);
                            }
                            continue;
                        }
                    };

                    if let Err(err) = self.state.request_converter.handle_request(request) {
                        cb(EisRequestSourceEvent::RequestError(err), &mut self.state)?;
                        return Ok(calloop::PostAction::Remove);
                    }
                    while let Some(request) = self.state.request_converter.next_request() {
                        let res = cb(EisRequestSourceEvent::Request(request), &mut self.state)?;
                        if res != calloop::PostAction::Continue {
                            return Ok(res);
                        }
                    }
                }

                Ok(calloop::PostAction::Continue)
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
    Request(crate::request::EisRequest),
    // Event source removes itself after error
    RequestError(crate::request::Error),
    ParseError(ParseError),
    IoError(io::Error),
    InvalidObject(u64),
}
