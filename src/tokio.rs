// TODO: Handle writable fd too?

use futures_core::stream::Stream;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::unix::AsyncFd;

use crate::{ei, PendingRequestResult};

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
                Ok(res) if res.is_eof() => {
                    return Poll::Ready(None);
                }
                Err(err) => {
                    return Poll::Ready(Some(Err(err)));
                }
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
