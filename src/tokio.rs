// TODO: Handle writable fd too?

use futures::stream::{Stream, StreamExt};
use std::{
    collections::HashMap,
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

pub async fn ei_handshake(
    events: &mut EiEventStream,
    name: &str,
    context_type: ei::handshake::ContextType,
    interfaces: &HashMap<&str, u32>,
) -> Result<HandshakeResp, HandshakeError> {
    let mut interface_versions = HashMap::new();

    while let Some(result) = events.next().await {
        dbg!(&result);
        let (handshake, request) = match result? {
            PendingRequestResult::Request(request) => match request {
                ei::Event::Handshake(handshake, request) => (handshake, request),
                _ => {
                    return Err(HandshakeError::Protocol(
                        "event on non-handshake object during handshake".to_string(),
                    ));
                }
            },
            PendingRequestResult::ProtocolError(protocol_error) => {
                return Err(HandshakeError::Protocol(protocol_error));
            }
            PendingRequestResult::InvalidObject(invalid_object) => {
                return Err(HandshakeError::Protocol(format!(
                    "invalid object: {}",
                    invalid_object
                )));
            }
        };

        match request {
            ei::handshake::Event::HandshakeVersion { version: _ } => {
                handshake.handshake_version(1);
                handshake.name(name);
                handshake.context_type(context_type);
                for (interface, version) in interfaces.iter() {
                    handshake.interface_version(interface, *version);
                }
                handshake.finish();

                // TODO Handle
                let _ = events.0.get_ref().flush();
            }
            ei::handshake::Event::InterfaceVersion { name, version } => {
                interface_versions.insert(name, version);
            }
            ei::handshake::Event::Connection { connection, serial } => {
                return Ok(HandshakeResp {
                    connection,
                    serial,
                    interface_versions,
                });
            }
        }
    }
    Err(HandshakeError::UnexpectedEof)
}

pub struct HandshakeResp {
    pub connection: ei::Connection,
    pub serial: u32,
    pub interface_versions: HashMap<String, u32>,
}

#[derive(Debug)]
pub enum HandshakeError {
    Io(io::Error),
    UnexpectedEof,
    Protocol(String),
}

impl From<io::Error> for HandshakeError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
