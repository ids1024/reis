use std::os::unix::io::{BorrowedFd, OwnedFd};

use crate::ByteStream;

#[derive(Debug)]
pub enum Arg<'a> {
    Uint32(u32),
    Int32(i32),
    Uint64(u64),
    Int64(i64),
    Float(f32),
    Fd(BorrowedFd<'a>),
    String(&'a str),
    NewId(u64),
    Id(u64),
}

impl<'a> Arg<'a> {
    pub fn write(&self, buf: &mut Vec<u8>, fds: &mut Vec<BorrowedFd<'a>>) {
        match self {
            Arg::Uint32(value) => buf.extend(value.to_ne_bytes()),
            Arg::Int32(value) => buf.extend(value.to_ne_bytes()),
            Arg::Uint64(value) => buf.extend(value.to_ne_bytes()),
            Arg::Int64(value) => buf.extend(value.to_ne_bytes()),
            Arg::Float(value) => buf.extend(value.to_ne_bytes()),
            Arg::Fd(value) => fds.push(*value),
            Arg::String(value) => {
                // Write 32-bit length, including NUL
                let len = value.len() as u32 + 1;
                buf.extend(len.to_ne_bytes());
                // Write contents of string, as UTF-8
                buf.extend(value.as_bytes());
                // Add NUL terminator
                buf.push(b'\0');
                // Pad to multiple of 32 bits
                if len % 4 != 0 {
                    buf.extend((0..4 - (len % 4)).map(|_| b'\0'));
                }
            }
            Arg::NewId(value) => buf.extend(value.to_ne_bytes()),
            Arg::Id(value) => buf.extend(value.to_ne_bytes()),
        }
    }
}

struct Id(u64);

struct NewId(u64);

pub trait OwnedArg: Sized {
    // TODO fds?
    fn parse(buf: &mut ByteStream) -> Option<Self>;
}

impl OwnedArg for u32 {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        Some(Self::from_ne_bytes(buf.read()?))
    }
}

impl OwnedArg for i32 {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        Some(Self::from_ne_bytes(buf.read()?))
    }
}

impl OwnedArg for u64 {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        Some(Self::from_ne_bytes(buf.read()?))
    }
}

impl OwnedArg for i64 {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        Some(Self::from_ne_bytes(buf.read()?))
    }
}

impl OwnedArg for f32 {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        Some(Self::from_ne_bytes(buf.read()?))
    }
}

// XXX how are fds grouped in stream?
impl OwnedArg for OwnedFd {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        // XXX error?
        buf.read_fd()
    }
}

impl OwnedArg for String {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        let mut len = u32::parse(buf)?;
        let bytes = buf.read_n(len as usize - 1)?; // Exclude NUL
                                                   // XXX error?
        let string = String::from_utf8(bytes.to_owned()).ok()?;
        buf.read_n(1)?; // NUL
        while len % 4 != 0 {
            // Padding
            len += 1;
            buf.read::<1>()?;
        }
        Some(string)
    }
}

impl OwnedArg for NewId {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        u64::parse(buf).map(Self)
    }
}

impl OwnedArg for Id {
    fn parse(buf: &mut ByteStream) -> Option<Self> {
        u64::parse(buf).map(Self)
    }
}

#[cfg(test)]
mod tests {
    // TODO add serialization/deserialization tests
}
