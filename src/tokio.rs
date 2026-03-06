//! Module containing [`tokio`] event streams.

// TODO: Handle writable fd too?

use futures_util::{Stream, StreamExt};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::unix::AsyncFd;

pub use crate::handshake::{HandshakeError, HandshakeResp};
use crate::{async_shared, ei, Error, PendingRequestResult};

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

impl Stream for EiEventStream {
    type Item = io::Result<PendingRequestResult<ei::Event>>; // XXX

    fn poll_next(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<<Self as Stream>::Item>> {
        // If we already have a pending event, return that
        if let Some(res) = async_shared::poll_pending_event(self.0.get_mut()) {
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
                    async_shared::poll_pending_event(self.0.get_mut()).unwrap_or(Poll::Pending)
                }
            }
        } else {
            Poll::Pending
        }
    }
}

// TODO rename EiProtoEventStream
/// EI convert event stream.
pub struct EiConvertEventStream(async_shared::EiConvertEventStream<EiEventStream>);

impl Stream for EiConvertEventStream {
    type Item = Result<crate::event::EiEvent, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<<Self as Stream>::Item>> {
        self.0.poll_next_unpin(context)
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
    async_shared::ei_handshake(events, name, context_type).await
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
        let converter = crate::event::EiEventConverter::new(events.0.get_ref(), resp);
        let stream =
            EiConvertEventStream(async_shared::EiConvertEventStream::new(events, converter));
        let connection = stream.0.converter.connection().clone();
        Ok((connection, stream))
    }
}
