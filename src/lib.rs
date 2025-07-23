#![forbid(unsafe_code)]

// TODO split up

use std::{env, path::PathBuf};

pub use wire::PendingRequestResult; // XXX types? names?
pub mod ei;
mod eiproto_ei;
mod eiproto_eis;
mod eiproto_enum;
pub mod eis;
mod error;
pub use error::Error;
pub mod event; // XXX reorganize?
pub mod handshake; // XXX ^
mod object;
pub mod request;
pub use object::Object;
mod util;
mod wire;

pub use wire::Interface;
pub use wire::ParseError;

#[cfg(feature = "calloop")]
//#[doc(hidden)] // TODO
pub mod calloop;
#[cfg(feature = "tokio")]
pub mod tokio;

// TODO versioning?

mod private {
    pub trait Sealed {}
}

// XXX
// Want to fallback to higher number if exists, on server?
// create on server, not client.
#[must_use]
pub fn default_socket_path() -> Option<PathBuf> {
    let mut path = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    path.push("eis-0");
    Some(path)
}
