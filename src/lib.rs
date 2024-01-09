#![forbid(unsafe_code)]

// TODO error type?
// TODO split up
// Implement handshake

use std::{env, path::PathBuf};

mod backend;
use backend::Backend;
pub use backend::{ConnectionReadResult, PendingRequestResult}; // XXX types? names?
pub mod ei;
mod eiproto_ei;
mod eiproto_eis;
mod eiproto_enum;
pub mod eis;
pub mod event; // XXX reorganize?
pub mod handshake; // XXX ^
mod object;
pub mod request; // XXX
pub use object::Object;
mod util;
mod wire;
pub use wire::{Interface, ParseError};

#[cfg(feature = "tokio")]
pub mod tokio;

// TODO versioning?

mod private {
    pub trait Sealed {}
}

// XXX
// Want to fallback to higher number if exists, on server?
// create on server, not client.
pub fn default_socket_path() -> Option<PathBuf> {
    let mut path = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    path.push("eis-0");
    Some(path)
}
