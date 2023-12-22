// Generic `EiHandshaker` can be used in async or sync code

use crate::{ei, ParseError};
use std::{collections::HashMap, io, mem};

pub struct HandshakeResp {
    pub connection: ei::Connection,
    pub serial: u32,
    pub interface_versions: HashMap<String, u32>,
}

#[derive(Debug)]
pub enum HandshakeError {
    Io(io::Error),
    UnexpectedEof,
    Parse(ParseError),
    InvalidObject(u64),
}

impl From<io::Error> for HandshakeError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

pub struct EiHandshaker<'a> {
    name: &'a str,
    context_type: ei::handshake::ContextType,
    interfaces: &'a HashMap<&'a str, u32>,
    interface_versions: HashMap<String, u32>,
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
            interface_versions: HashMap::new(),
        }
    }

    pub fn handle_event(
        &mut self,
        event: ei::Event,
    ) -> Result<Option<HandshakeResp>, HandshakeError> {
        let (handshake, event) = match event {
            ei::Event::Handshake(handshake, event) => (handshake, event),
            _ => {
                panic!("Event on non-handshake object during handshake. `ei_handshake` called after handshake?");
            }
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
                self.interface_versions.insert(name, version);
                Ok(None)
            }
            ei::handshake::Event::Connection { connection, serial } => Ok(Some(HandshakeResp {
                connection,
                serial,
                interface_versions: mem::take(&mut self.interface_versions),
            })),
        }
    }
}
