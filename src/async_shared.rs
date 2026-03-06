// Async code shared between tokio and async-io

use futures_util::{Stream, StreamExt};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

pub use crate::handshake::HandshakeResp;
use crate::{ei, handshake::EiHandshaker, Error, PendingRequestResult};

pub(crate) fn poll_pending_event(
    context: &ei::Context,
) -> Option<Poll<Option<io::Result<PendingRequestResult<ei::Event>>>>> {
    Some(Poll::Ready(Some(Ok(context.pending_event()?))))
}

pub(crate) struct EiConvertEventStream<
    S: Stream<Item = io::Result<PendingRequestResult<ei::Event>>> + Unpin,
> {
    inner: S,
    pub(crate) converter: crate::event::EiEventConverter,
}

impl<S: Stream<Item = io::Result<PendingRequestResult<ei::Event>>> + Unpin>
    EiConvertEventStream<S>
{
    pub(crate) fn new(inner: S, converter: crate::event::EiEventConverter) -> Self {
        Self { inner, converter }
    }
}

impl<S: Stream<Item = io::Result<PendingRequestResult<ei::Event>>> + Unpin> Stream
    for EiConvertEventStream<S>
{
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

pub async fn ei_handshake<S>(
    events: &mut S,
    name: &str,
    context_type: ei::handshake::ContextType,
) -> Result<HandshakeResp, Error>
where
    S: Stream<Item = io::Result<PendingRequestResult<ei::Event>>> + Unpin,
{
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
