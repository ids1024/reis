// Generic `EiHandshaker` can be used in async or sync code

use crate::{ei, eis, util, ParseError, PendingRequestResult};
use std::{collections::HashMap, error, fmt, io, mem};

pub struct HandshakeResp {
    pub connection: ei::Connection,
    pub serial: u32,
    pub negotiated_interfaces: HashMap<String, u32>,
}

#[derive(Debug)]
pub enum HandshakeError {
    Io(io::Error),
    Parse(ParseError),
    InvalidObject(u64),
    NonHandshakeEvent,
    MissingInterface,
    DuplicateEvent,
    NoContextType,
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Parse(err) => write!(f, "parse error: {}", err),
            Self::InvalidObject(id) => write!(f, "invalid object {} during handshake", id),
            Self::NonHandshakeEvent => write!(f, "non-handshake event during handshake"),
            Self::MissingInterface => write!(f, "missing required interface"),
            Self::DuplicateEvent => write!(f, "duplicate event during handshake"),
            Self::NoContextType => write!(f, "no `context_type` sent in handshake"),
        }
    }
}

impl error::Error for HandshakeError {}

impl From<io::Error> for HandshakeError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

pub struct EiHandshaker<'a> {
    name: &'a str,
    context_type: ei::handshake::ContextType,
    interfaces: &'a HashMap<&'a str, u32>,
    negotiated_interfaces: HashMap<String, u32>,
}

impl<'a> EiHandshaker<'a> {
    pub fn new(
        name: &'a str,
        context_type: ei::handshake::ContextType,
        interfaces: &'a HashMap<&'a str, u32>,
    ) -> Self {
        Self {
            name,
            context_type,
            interfaces,
            negotiated_interfaces: HashMap::new(),
        }
    }

    pub fn handle_event(
        &mut self,
        event: ei::Event,
    ) -> Result<Option<HandshakeResp>, HandshakeError> {
        let ei::Event::Handshake(handshake, event) = event else {
            return Err(HandshakeError::NonHandshakeEvent);
        };
        match event {
            ei::handshake::Event::HandshakeVersion { version: _ } => {
                handshake.handshake_version(1);
                handshake.name(self.name);
                handshake.context_type(self.context_type);
                for (interface, version) in self.interfaces.iter() {
                    handshake.interface_version(interface, *version);
                }
                handshake.finish();

                if let Some(backend) = handshake.0.backend() {
                    // TODO Handle result
                    let _ = backend.flush();
                }

                Ok(None)
            }
            ei::handshake::Event::InterfaceVersion { name, version } => {
                self.negotiated_interfaces.insert(name, version);
                Ok(None)
            }
            ei::handshake::Event::Connection { connection, serial } => Ok(Some(HandshakeResp {
                connection,
                serial,
                negotiated_interfaces: mem::take(&mut self.negotiated_interfaces),
            })),
        }
    }
}

pub(crate) fn request_result<T>(result: PendingRequestResult<T>) -> Result<T, HandshakeError> {
    match result {
        PendingRequestResult::Request(request) => Ok(request),
        PendingRequestResult::ParseError(parse_error) => {
            return Err(HandshakeError::Parse(parse_error));
        }
        PendingRequestResult::InvalidObject(invalid_object) => {
            return Err(HandshakeError::InvalidObject(invalid_object));
        }
    }
}

pub fn ei_handshake_blocking(
    context: &ei::Context,
    name: &str,
    context_type: ei::handshake::ContextType,
    interfaces: &HashMap<&str, u32>,
) -> Result<HandshakeResp, HandshakeError> {
    let mut handshaker = EiHandshaker::new(name, context_type, interfaces);
    loop {
        util::poll_readable(context)?;
        context.read()?;
        while let Some(result) = context.pending_event() {
            let request = request_result(result)?;
            if let Some(resp) = handshaker.handle_event(request)? {
                return Ok(resp);
            }
        }
    }
}

pub struct EisHandshakeResp {
    pub connection: eis::Connection,
    pub name: Option<String>,
    pub context_type: eis::handshake::ContextType,
    pub negotiated_interfaces: HashMap<String, u32>,
}

pub struct EisHandshaker<'a> {
    interfaces: &'a HashMap<&'a str, u32>,
    name: Option<String>,
    context_type: Option<eis::handshake::ContextType>,
    negotiated_interfaces: HashMap<String, u32>,
    initial_serial: u32,
}

impl<'a> EisHandshaker<'a> {
    pub fn new(
        context: &eis::Context,
        interfaces: &'a HashMap<&'a str, u32>,
        initial_serial: u32,
    ) -> Self {
        let handshake = context.handshake();
        handshake.handshake_version(1);
        // XXX error handling?
        let _ = context.flush();

        Self {
            interfaces,
            initial_serial,
            name: None,
            context_type: None,
            negotiated_interfaces: HashMap::new(),
        }
    }

    pub fn handle_request(
        &mut self,
        request: eis::Request,
    ) -> Result<Option<EisHandshakeResp>, HandshakeError> {
        let eis::Request::Handshake(handshake, request) = request else {
            return Err(HandshakeError::NonHandshakeEvent);
        };
        match request {
            eis::handshake::Request::HandshakeVersion { version: _ } => {}
            eis::handshake::Request::Name { name } => {
                if self.name.is_some() {
                    return Err(HandshakeError::DuplicateEvent);
                }
                self.name = Some(name);
            }
            eis::handshake::Request::ContextType { context_type } => {
                if self.context_type.is_some() {
                    return Err(HandshakeError::DuplicateEvent);
                }
                self.context_type = Some(context_type);
            }
            eis::handshake::Request::InterfaceVersion { name, version } => {
                if let Some((interface, server_version)) =
                    self.interfaces.get_key_value(name.as_str())
                {
                    self.negotiated_interfaces
                        .insert(interface.to_string(), version.min(*server_version));
                }
            }
            eis::handshake::Request::Finish => {
                for (interface, version) in self.negotiated_interfaces.iter() {
                    handshake.interface_version(interface, *version);
                }

                if !self.negotiated_interfaces.contains_key("ei_connection")
                    || !self.negotiated_interfaces.contains_key("ei_pingpong")
                    || !self.negotiated_interfaces.contains_key("ei_callback")
                {
                    return Err(HandshakeError::MissingInterface);
                }

                let connection = handshake.connection(self.initial_serial, 1);

                let Some(context_type) = self.context_type else {
                    return Err(HandshakeError::NoContextType);
                };

                return Ok(Some(EisHandshakeResp {
                    connection,
                    name: self.name.clone(),
                    context_type,
                    negotiated_interfaces: mem::take(&mut self.negotiated_interfaces),
                }));
            }
        }
        Ok(None)
    }
}

// Does handshake always succeed? When does it prompt, if needed?
// Look at libei, improve any documentation.
