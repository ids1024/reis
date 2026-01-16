//! Module implementing the EI protocol handshake.
//!
//! The generic [`EiHandshaker`] can be used in async and sync code.

use crate::{ei, eis, util, Error, PendingRequestResult};
use std::{collections::HashMap, error, fmt, mem, sync::OnceLock};

fn interfaces() -> &'static HashMap<&'static str, u32> {
    fn iface<I: ei::Interface>() -> (&'static str, u32) {
        (I::NAME, I::VERSION)
    }
    static INTERFACES: OnceLock<HashMap<&'static str, u32>> = OnceLock::new();
    INTERFACES.get_or_init(|| {
        [
            iface::<ei::Connection>(),
            iface::<ei::Callback>(),
            iface::<ei::Pingpong>(),
            iface::<ei::Seat>(),
            iface::<ei::Device>(),
            iface::<ei::Pointer>(),
            iface::<ei::PointerAbsolute>(),
            iface::<ei::Scroll>(),
            iface::<ei::Button>(),
            iface::<ei::Keyboard>(),
            iface::<ei::Touchscreen>(),
        ]
        .into_iter()
        .collect()
    })
}

/// Handshake response.
#[derive(Clone, Debug)]
pub struct HandshakeResp {
    /// Global `ei_connection` singleton object.
    pub connection: ei::Connection,
    /// Serial number of `ei_handshake.connection`.
    pub serial: u32,
    /// Interfaces along with their versions negotiated in the handshake.
    pub negotiated_interfaces: HashMap<String, u32>,
}

/// Error during handshake.
#[derive(Debug)]
pub enum HandshakeError {
    /// Invalid object ID.
    InvalidObject(u64),
    /// Non-handshake event.
    NonHandshakeEvent,
    /// Missing required interface.
    MissingInterface,
    /// Duplicate event.
    DuplicateEvent,
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidObject(id) => write!(f, "invalid object {id} during handshake"),
            Self::NonHandshakeEvent => write!(f, "non-handshake event during handshake"),
            Self::MissingInterface => write!(f, "missing required interface"),
            Self::DuplicateEvent => write!(f, "duplicate event during handshake"),
        }
    }
}

impl error::Error for HandshakeError {}

/// Implementation of the EI protocol handshake on the client side.
pub struct EiHandshaker<'a> {
    name: &'a str,
    context_type: ei::handshake::ContextType,
    negotiated_interfaces: HashMap<String, u32>,
}

impl<'a> EiHandshaker<'a> {
    /// Creates a client-side handshaker.
    #[must_use]
    pub fn new(name: &'a str, context_type: ei::handshake::ContextType) -> Self {
        Self {
            name,
            context_type,
            negotiated_interfaces: HashMap::new(),
        }
    }

    /// Handles the given event, possibly returning a filled handshake response.
    ///
    /// # Errors
    ///
    /// The errors returned are protocol violations.
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
                for (interface, version) in interfaces() {
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

pub(crate) fn request_result<T>(result: PendingRequestResult<T>) -> Result<T, Error> {
    match result {
        PendingRequestResult::Request(request) => Ok(request),
        PendingRequestResult::ParseError(parse_error) => Err(Error::Parse(parse_error)),
        PendingRequestResult::InvalidObject(invalid_object) => {
            Err(HandshakeError::InvalidObject(invalid_object).into())
        }
    }
}

/// Executes the handshake in blocking mode.
///
/// # Errors
///
/// Will return `Err` if there is an I/O error or a protocol violation.
pub fn ei_handshake_blocking(
    context: &ei::Context,
    name: &str,
    context_type: ei::handshake::ContextType,
) -> Result<HandshakeResp, Error> {
    let mut handshaker = EiHandshaker::new(name, context_type);
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

/// Handshake response.
#[derive(Clone, Debug)]
pub struct EisHandshakeResp {
    /// Global `ei_connection` singleton object.
    pub connection: eis::Connection,
    /// Name of client.
    pub name: Option<String>,
    /// Context type of connection.
    pub context_type: eis::handshake::ContextType,
    /// Interfaces along with their versions negotiated in the handshake.
    pub negotiated_interfaces: HashMap<String, u32>,
}

/// Implementation of the EI protocol handshake on the server side.
#[derive(Debug)]
pub struct EisHandshaker {
    name: Option<String>,
    context_type: Option<eis::handshake::ContextType>,
    negotiated_interfaces: HashMap<String, u32>,
    initial_serial: u32,
}

impl EisHandshaker {
    /// Creates a server-side handshaker.
    #[must_use]
    pub fn new(context: &eis::Context, initial_serial: u32) -> Self {
        let handshake = context.handshake();
        handshake.handshake_version(1);
        // XXX error handling?
        let _ = context.flush();

        Self {
            initial_serial,
            name: None,
            context_type: None,
            negotiated_interfaces: HashMap::new(),
        }
    }

    /// Handles the given request, possibly returning a filled handshake response.
    ///
    /// # Errors
    ///
    /// The errors returned are protocol violations.
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
                if let Some((interface, server_version)) = interfaces().get_key_value(name.as_str())
                {
                    self.negotiated_interfaces
                        .insert((*interface).to_owned(), version.min(*server_version));
                }
            }
            eis::handshake::Request::Finish => {
                for (interface, version) in &self.negotiated_interfaces {
                    handshake.interface_version(interface, *version);
                }

                if !self.negotiated_interfaces.contains_key("ei_connection")
                    || !self.negotiated_interfaces.contains_key("ei_pingpong")
                    || !self.negotiated_interfaces.contains_key("ei_callback")
                {
                    return Err(HandshakeError::MissingInterface);
                }

                let connection = handshake.connection(self.initial_serial, 1);

                // Protocol spec says `context_type` is optional, defaults to receiver
                let context_type = self
                    .context_type
                    .unwrap_or(ei::handshake::ContextType::Receiver);

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
