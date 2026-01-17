// Types representing arguments to requests/events
// Used in parsing and serialization.

use std::{
    fmt,
    iter::Extend,
    os::unix::io::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
};

use super::{ByteStream, ParseError};

/// An argument in an event or a request.
#[allow(dead_code)]
#[derive(Debug)]
pub enum Arg<'a> {
    Uint32(u32),
    Int32(i32),
    Uint64(u64),
    Int64(i64),
    Float(f32),
    Fd(BorrowedFd<'a>),
    String(Option<&'a str>),
    NewId(u64),
    Id(u64),
}

impl fmt::Display for Arg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uint32(value) => write!(f, "{value}"),
            Self::Int32(value) => write!(f, "{value}"),
            Self::Uint64(value) => write!(f, "{value}"),
            Self::Int64(value) => write!(f, "{value}"),
            Self::Float(value) => write!(f, "{value}"),
            Self::Fd(value) => write!(f, "fd {}", value.as_raw_fd()),
            Self::String(value) => write!(f, "{value:?}"),
            Self::NewId(value) => write!(f, "new_id {value:x}"),
            Self::Id(value) => write!(f, "id {value:x}"),
        }
    }
}

impl Arg<'_> {
    pub fn write<T, U>(&self, buf: &mut T, fds: &mut U)
    where
        T: Extend<u8>,
        U: Extend<OwnedFd>,
    {
        match self {
            Arg::Uint32(value) => buf.extend(value.to_ne_bytes()),
            Arg::Int32(value) => buf.extend(value.to_ne_bytes()),
            Arg::Uint64(value) | Arg::NewId(value) | Arg::Id(value) => {
                buf.extend(value.to_ne_bytes());
            }
            Arg::Int64(value) => buf.extend(value.to_ne_bytes()),
            Arg::Float(value) => buf.extend(value.to_ne_bytes()),
            // XXX unwrap?
            Arg::Fd(value) => fds.extend([value.try_clone_to_owned().unwrap()]),
            Arg::String(None) => {
                buf.extend(0u32.to_ne_bytes());
            }
            Arg::String(Some(value)) => {
                // Write 32-bit length, including NUL
                let len = value.len() as u32 + 1;
                buf.extend(len.to_ne_bytes());
                // Write contents of string, as UTF-8
                buf.extend(value.as_bytes().iter().copied());
                // Add NUL terminator
                buf.extend([b'\0']);
                // Pad to multiple of 32 bits
                if len % 4 != 0 {
                    buf.extend((0..4 - (len % 4)).map(|_| b'\0'));
                }
            }
        }
    }
}

pub(crate) trait OwnedArg: Sized {
    fn parse(buf: &mut ByteStream) -> Result<Self, ParseError>;

    fn as_arg(&self) -> Arg<'_>;

    // For enum types, this returns the name of the enum and variant
    #[allow(dead_code)]
    fn enum_name(&self) -> Option<(&'static str, &'static str)> {
        None
    }
}

impl OwnedArg for u32 {
    fn parse(buf: &mut ByteStream) -> Result<Self, ParseError> {
        Ok(Self::from_ne_bytes(buf.read()?))
    }

    fn as_arg(&self) -> Arg<'_> {
        Arg::Uint32(*self)
    }
}

impl OwnedArg for i32 {
    fn parse(buf: &mut ByteStream) -> Result<Self, ParseError> {
        Ok(Self::from_ne_bytes(buf.read()?))
    }

    fn as_arg(&self) -> Arg<'_> {
        Arg::Int32(*self)
    }
}

impl OwnedArg for u64 {
    fn parse(buf: &mut ByteStream) -> Result<Self, ParseError> {
        Ok(Self::from_ne_bytes(buf.read()?))
    }

    fn as_arg(&self) -> Arg<'_> {
        Arg::Uint64(*self)
    }
}

impl OwnedArg for i64 {
    fn parse(buf: &mut ByteStream) -> Result<Self, ParseError> {
        Ok(Self::from_ne_bytes(buf.read()?))
    }

    fn as_arg(&self) -> Arg<'_> {
        Arg::Int64(*self)
    }
}

impl OwnedArg for f32 {
    fn parse(buf: &mut ByteStream) -> Result<Self, ParseError> {
        Ok(Self::from_ne_bytes(buf.read()?))
    }

    fn as_arg(&self) -> Arg<'_> {
        Arg::Float(*self)
    }
}

impl OwnedArg for OwnedFd {
    fn parse(buf: &mut ByteStream) -> Result<Self, ParseError> {
        buf.read_fd()
    }

    fn as_arg(&self) -> Arg<'_> {
        Arg::Fd(self.as_fd())
    }
}

impl OwnedArg for Option<String> {
    fn parse(buf: &mut ByteStream) -> Result<Self, ParseError> {
        let mut len = u32::parse(buf)?;
        if len == 0 {
            return Ok(None);
        }
        let bytes = buf.read_n(len as usize - 1)?; // Exclude NUL
        let string = String::from_utf8(bytes.collect())?;
        buf.read_n(1)?.next(); // NUL
        while len % 4 != 0 {
            // Padding
            len += 1;
            buf.read::<1>()?;
        }
        Ok(Some(string))
    }

    fn as_arg(&self) -> Arg<'_> {
        Arg::String(self.as_deref())
    }
}

impl OwnedArg for String {
    fn parse(buf: &mut ByteStream) -> Result<Self, ParseError> {
        Option::<String>::parse(buf)?.ok_or(ParseError::InvalidNull)
    }

    fn as_arg(&self) -> Arg<'_> {
        Arg::String(Some(self))
    }
}

#[cfg(test)]
mod tests {
    // TODO add serialization/deserialization tests
}
