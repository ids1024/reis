//! Reis 🍚 provides a Rust version of EI 🥚 and EIS 🍨 for emulated input on Wayland.
//!
//! See the upstream project [libei](https://gitlab.freedesktop.org/libinput/libei) for more information.
//!
//! This library is currently **incomplete** and subject to change. It should probably do more to provide a more high level API that handles the things a client/server needs to deal with.
//!
//! Setting the env var `REIS_DEBUG` will make the library print ei messages it sends and receives.
//!
//! # Features
//!
//! `reis` has the following Cargo features:
//!
//! - `tokio`: Enables tokio support for clients.
//! - `calloop`: Enables calloop sources for EIS implementations. Somewhat experimental and
//!   incomplete.

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
/// Returns the default socket path for EIS.
#[must_use]
pub fn default_socket_path() -> Option<PathBuf> {
    let mut path = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    path.push("eis-0");
    Some(path)
}
