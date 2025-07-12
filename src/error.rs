use crate::{event::EventError, handshake::HandshakeError, ParseError};
use std::{fmt, io};

/// An error coming from the `reis` crate
#[derive(Debug)]
pub enum Error {
    UnexpectedHandshakeEvent,
    InvalidInterfaceVersion(&'static str, u32),
    // TODO better error type here?
    Event(EventError),
    Parse(ParseError),
    Handshake(HandshakeError),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UnexpectedHandshakeEvent => write!(f, "unexpected handshake event"),
            Self::InvalidInterfaceVersion(interface, version) => {
                write!(f, "invalid version {version} for interface '{interface}'")
            }
            Self::Event(err) => write!(f, "event error: {err}"),
            Self::Io(err) => write!(f, "IO error: {err}"),
            Self::Handshake(err) => write!(f, "handshake error: {err}"),
            Self::Parse(err) => write!(f, "parse error: {err}"),
        }
    }
}

impl From<EventError> for Error {
    fn from(err: EventError) -> Self {
        Self::Event(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Self::Parse(err)
    }
}

impl From<HandshakeError> for Error {
    fn from(err: HandshakeError) -> Self {
        Self::Handshake(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl std::error::Error for Error {}
