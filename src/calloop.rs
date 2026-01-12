//! Module containing [`calloop`] sources.

// TODO Define an event source that reads socket and produces eis::Event
// - is it easy to compose and wrap with handshaker, event handler?
// produce an event of some kind on disconnect/eof?

use calloop::{generic::Generic, Interest, Mode, PostAction, Readiness, Token, TokenFactory};
use std::io;

use crate::{
    eis,
    request::{self, Connection, EisRequestConverter},
    Error, PendingRequestResult,
};

/// [`calloop`] source that receives EI connections by listening on a socket.
#[derive(Debug)]
pub struct EisListenerSource {
    source: Generic<eis::Listener>,
}

impl EisListenerSource {
    /// Creates a new EIS listener source.
    #[must_use]
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
struct ConnectedContextState {
    context: eis::Context,
    request_converter: request::EisRequestConverter,
    handle: Connection,
}

impl ConnectedContextState {
    fn process<F>(&mut self, mut cb: F) -> io::Result<PostAction>
    where
        F: FnMut(Result<EisRequestSourceEvent, Error>, &mut Connection) -> io::Result<PostAction>,
    {
        if let Err(err) = self.context.read() {
            cb(Err(Error::Io(err)), &mut self.handle)?;
            return Ok(calloop::PostAction::Remove);
        }

        while let Some(result) = self.context.pending_request() {
            let request = match result {
                PendingRequestResult::Request(request) => request,
                PendingRequestResult::ParseError(err) => {
                    cb(Err(Error::Parse(err)), &mut self.handle)?;
                    return Ok(calloop::PostAction::Remove);
                }
                PendingRequestResult::InvalidObject(object_id) => {
                    log::debug!("reis: Failed to find object {object_id}");
                    // Only send if object ID is in range?
                    self.handle
                        .connection()
                        .invalid_object(self.handle.last_serial(), object_id);
                    continue;
                }
            };

            if let Err(err) = self.request_converter.handle_request(request) {
                cb(Err(err), &mut self.handle)?;
                return Ok(calloop::PostAction::Remove);
            }
            while let Some(request) = self.request_converter.next_request() {
                let res = cb(
                    Ok(EisRequestSourceEvent::Request(request)),
                    &mut self.handle,
                )?;
                if res != calloop::PostAction::Continue {
                    return Ok(res);
                }
            }
        }

        Ok(calloop::PostAction::Continue)
    }
}

fn process_handshake(
    handshaker: &mut crate::handshake::EisHandshaker,
    context: &eis::Context,
) -> Result<Option<ConnectedContextState>, Error> {
    context.read()?;

    while let Some(result) = context.pending_request() {
        let request = crate::handshake::request_result(result)?;
        if let Some(resp) = handshaker.handle_request(request)? {
            let request_converter = EisRequestConverter::new(context, resp, 1);

            let connected_state = ConnectedContextState {
                context: context.clone(),
                handle: request_converter.handle().clone(),
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
    Handshake(crate::handshake::EisHandshaker),
    Connected(ConnectedContextState),
}

/// [`calloop`] source that receives EI protocol requests.
#[derive(Debug)]
pub struct EisRequestSource {
    source: Generic<eis::Context>,
    state: State,
}

impl EisRequestSource {
    /// Creates a new EIS request source.
    #[must_use]
    pub fn new(context: eis::Context, initial_serial: u32) -> Self {
        let handshaker = crate::handshake::EisHandshaker::new(&context, initial_serial);
        Self {
            source: Generic::new(context, Interest::READ, Mode::Level),
            state: State::Handshake(handshaker),
        }
    }
}

impl calloop::EventSource for EisRequestSource {
    type Event = Result<EisRequestSourceEvent, Error>;
    type Metadata = Connection;
    type Ret = io::Result<PostAction>;
    type Error = io::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut cb: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut(Self::Event, &mut Connection) -> io::Result<PostAction>,
    {
        self.source
            .process_events(readiness, token, |_readiness, context| {
                match &mut self.state {
                    State::Handshake(handshaker) => {
                        if let Some(res) = process_handshake(handshaker, context).transpose() {
                            match res {
                                Ok(mut state) => {
                                    let res = cb(
                                        Ok(EisRequestSourceEvent::Connected),
                                        &mut state.handle,
                                    )?;
                                    self.state = State::Connected(state);
                                    Ok(res)
                                }
                                Err(err) => {
                                    // TODO return handshake errors?
                                    eprintln!("Client handshake failed: {err}");
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
/// Event returned by [`EisRequestSource`].
#[derive(Debug)]
pub enum EisRequestSourceEvent {
    /// Handshake has finished.
    Connected,
    /// High-level request to EIS.
    Request(request::EisRequest),
}
