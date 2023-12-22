// TODO: Handle writable fd too?

use futures::stream::{Stream, StreamExt};
use std::{
    collections::HashMap,
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::unix::AsyncFd;

pub use crate::handshake::{HandshakeError, HandshakeResp};
use crate::{ei, handshake::EiHandshaker, ParseError, PendingRequestResult};

// XXX make this ei::EventStream?
pub struct EiEventStream(AsyncFd<ei::Context>);

impl EiEventStream {
    pub fn new(context: ei::Context) -> io::Result<Self> {
        AsyncFd::with_interest(context, tokio::io::Interest::READABLE).map(Self)
    }
}

fn poll_pending_event(
    context: &mut ei::Context,
) -> Option<Poll<Option<io::Result<PendingRequestResult<ei::Event>>>>> {
    Some(Poll::Ready(Some(Ok(context.pending_event()?))))
}

impl Stream for EiEventStream {
    type Item = io::Result<PendingRequestResult<ei::Event>>; // XXX

    fn poll_next(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<<Self as Stream>::Item>> {
        // If we already have a pending event, return that
        if let Some(res) = poll_pending_event(self.0.get_mut()) {
            return res;
        }
        if let Poll::Ready(guard) = Pin::new(&self.0).poll_read_ready(context) {
            let mut guard = match guard {
                Ok(guard) => guard,
                Err(err) => {
                    return Poll::Ready(Some(Err(err)));
                }
            };
            match guard.get_inner().read() {
                Ok(res) if res.is_eof() => Poll::Ready(None),
                Err(err) => Poll::Ready(Some(Err(err))),
                Ok(_) => {
                    // `Backend::read()` reads until `WouldBlock`, EOF, or error
                    guard.clear_ready();
                    poll_pending_event(self.0.get_mut()).unwrap_or(Poll::Pending)
                }
            }
        } else {
            Poll::Pending
        }
    }
}

#[derive(Debug)]
pub enum EiConvertEventStreamError {
    Io(io::Error),
    Parse(ParseError),
    // TODO better error type here?
    Event(crate::event::Error),
}

// TODO rename EiProtoEventStream
pub struct EiConvertEventStream {
    inner: EiEventStream,
    converter: crate::event::EiEventConverter,
}

impl EiConvertEventStream {
    pub fn new(inner: EiEventStream) -> Self {
        Self {
            inner,
            converter: Default::default(),
        }
    }
}

impl Stream for EiConvertEventStream {
    type Item = Result<crate::event::EiEvent, EiConvertEventStreamError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<<Self as Stream>::Item>> {
        if let Some(event) = self.converter.next_event() {
            return Poll::Ready(Some(Ok(event)));
        }
        while let Poll::Ready(res) = Pin::new(&mut self.inner).poll_next(context) {
            match res {
                Some(Ok(res)) => match res {
                    PendingRequestResult::Request(event) => {
                        if let Err(err) = self.converter.handle_event(event) {
                            return Poll::Ready(Some(Err(EiConvertEventStreamError::Event(err))));
                        }
                        if let Some(event) = self.converter.next_event() {
                            return Poll::Ready(Some(Ok(event)));
                        }
                    }
                    PendingRequestResult::ParseError(err) => {
                        return Poll::Ready(Some(Err(EiConvertEventStreamError::Parse(err))));
                    }
                    // TODO log?
                    PendingRequestResult::InvalidObject(_object_id) => {}
                },
                Some(Err(err)) => {
                    return Poll::Ready(Some(Err(EiConvertEventStreamError::Io(err))));
                }
                None => {
                    return Poll::Ready(None);
                }
            }
        }
        Poll::Pending
    }
}

pub async fn ei_handshake(
    events: &mut EiEventStream,
    name: &str,
    context_type: ei::handshake::ContextType,
    interfaces: &HashMap<&str, u32>,
) -> Result<HandshakeResp, HandshakeError> {
    let mut handshaker = EiHandshaker::new(name, context_type, interfaces);
    while let Some(result) = events.next().await {
        let request = match result? {
            PendingRequestResult::Request(request) => request,
            PendingRequestResult::ParseError(parse_error) => {
                return Err(HandshakeError::Parse(parse_error));
            }
            PendingRequestResult::InvalidObject(invalid_object) => {
                return Err(HandshakeError::InvalidObject(invalid_object));
            }
        };

        if let Some(resp) = handshaker.handle_event(request)? {
            return Ok(resp);
        }
    }
    Err(HandshakeError::UnexpectedEof)
}
