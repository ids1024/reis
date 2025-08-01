//! Module containing [`tokio`] event streams.

// TODO: Handle writable fd too?

use futures::stream::{Stream, StreamExt};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::unix::AsyncFd;

pub use crate::handshake::{HandshakeError, HandshakeResp};
use crate::{ei, handshake::EiHandshaker, Error, PendingRequestResult};

// XXX make this ei::EventStream?
/// Stream of `ei::Event`s.
pub struct EiEventStream(AsyncFd<ei::Context>);

impl EiEventStream {
    /// Creates a new event stream.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the underlying async file descriptor registration fails.
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
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => Poll::Ready(None),
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

// TODO rename EiProtoEventStream
/// EI convert event stream.
pub struct EiConvertEventStream {
    inner: EiEventStream,
    converter: crate::event::EiEventConverter,
}

impl EiConvertEventStream {
    fn new(inner: EiEventStream, handshake_resp: HandshakeResp) -> Self {
        Self {
            converter: crate::event::EiEventConverter::new(inner.0.get_ref(), handshake_resp),
            inner,
        }
    }
}

impl Stream for EiConvertEventStream {
    type Item = Result<crate::event::EiEvent, Error>;

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
                            return Poll::Ready(Some(Err(err.into())));
                        }
                        if let Some(event) = self.converter.next_event() {
                            return Poll::Ready(Some(Ok(event)));
                        }
                    }
                    PendingRequestResult::ParseError(err) => {
                        return Poll::Ready(Some(Err(err.into())));
                    }
                    // TODO log?
                    PendingRequestResult::InvalidObject(_object_id) => {}
                },
                Some(Err(err)) => {
                    return Poll::Ready(Some(Err(err.into())));
                }
                None => {
                    return Poll::Ready(None);
                }
            }
        }
        Poll::Pending
    }
}

/// Executes the handshake in async mode.
///
/// # Errors
///
/// Will return `Err` if there is an I/O error or a protocol violation.
pub async fn ei_handshake(
    events: &mut EiEventStream,
    name: &str,
    context_type: ei::handshake::ContextType,
) -> Result<HandshakeResp, Error> {
    let mut handshaker = EiHandshaker::new(name, context_type);
    while let Some(result) = events.next().await {
        let request = crate::handshake::request_result(result?)?;
        if let Some(resp) = handshaker.handle_event(request)? {
            return Ok(resp);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "unexpected EOF reading ei socket",
    )
    .into())
}

impl ei::Context {
    /// Executes the handshake in async mode.
    ///
    /// # Errors
    ///
    /// Will return `Err` if there is an I/O error or a protocol violation.
    pub async fn handshake_tokio(
        &self,
        name: &str,
        context_type: ei::handshake::ContextType,
    ) -> Result<(crate::event::Connection, EiConvertEventStream), Error> {
        let mut events = EiEventStream::new(self.clone())?;
        let resp = ei_handshake(&mut events, name, context_type).await?;
        let stream = EiConvertEventStream::new(events, resp);
        let connection = stream.converter.connection().clone();
        Ok((connection, stream))
    }
}
