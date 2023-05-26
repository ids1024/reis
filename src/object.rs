use std::{fmt, sync::Arc};

use crate::{Arg, Backend, Interface};

#[derive(Clone)]
pub struct Object {
    // TODO use weak, like wayland-rs?
    connection: Arc<Backend>,
    id: u64,
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Object(_, {})", self.id)
    }
}

impl Object {
    pub(crate) fn new(connection: Arc<Backend>, id: u64) -> Self {
        Self { connection, id }
    }

    pub fn connection(&self) -> &Arc<Backend> {
        &self.connection
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn request(&self, opcode: u32, args: &[Arg]) -> rustix::io::Result<()> {
        self.connection.request(self.id, opcode, args)
    }

    pub(crate) fn downcast_unchecked<T: Interface>(self) -> T {
        T::new_unchecked(self)
    }

    // XXX test ei vs ei
    pub fn downcast<T: Interface>(self) -> Option<T> {
        let (interface, _version) = self.connection.object_interface(self.id)?;
        if &interface == T::NAME {
            Some(self.downcast_unchecked())
        } else {
            None
        }
    }
}
