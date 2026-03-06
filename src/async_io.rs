//! Module containing [`async_io`] event streams.
//!
use async_io::Async;
use futures::stream::{Stream, StreamExt};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

pub use crate::handshake::{HandshakeError, HandshakeResp};
use crate::{async_shared, ei, Error, PendingRequestResult};

/// Stream of `ei::Event`s.
pub struct EiEventStream(Async<ei::Context>);

impl EiEventStream {
    /// Creates a new event stream.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the underlying async file descriptor registration fails.
    pub fn new(context: ei::Context) -> io::Result<Self> {
        Async::new(context).map(Self)
    }
}

impl Stream for EiEventStream {
    type Item = io::Result<PendingRequestResult<ei::Event>>; // XXX

    fn poll_next(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<<Self as Stream>::Item>> {
        // If we already have a pending event, return that
        if let Some(res) = async_shared::poll_pending_event(self.0.get_ref()) {
            return res;
        }
        if let Poll::Ready(res) = Pin::new(&self.0).poll_readable(context) {
            if let Err(err) = res {
                return Poll::Ready(Some(Err(err)));
            }
            match self.0.get_ref().read() {
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => Poll::Ready(None),
                Err(err) => Poll::Ready(Some(Err(err))),
                Ok(_) => {
                    // `Backend::read()` reads until `WouldBlock`, EOF, or error
                    async_shared::poll_pending_event(self.0.get_ref()).unwrap_or(Poll::Pending)
                }
            }
        } else {
            Poll::Pending
        }
    }
}

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
    pub async fn handshake_async_io(
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
