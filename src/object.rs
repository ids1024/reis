use std::fmt;

use crate::{Arg, Backend, Interface};

#[derive(Clone)]
pub struct Object {
    // TODO use weak, like wayland-rs?
    backend: Backend,
    client_side: bool,
    id: u64,
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Object(_, {})", self.id)
    }
}

impl Object {
    pub(crate) fn new(backend: Backend, id: u64, client_side: bool) -> Self {
        Self {
            backend,
            id,
            client_side,
        }
    }

    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn request(&self, opcode: u32, args: &[Arg]) {
        self.backend.request(self.id, opcode, args);
    }

    pub(crate) fn downcast_unchecked<T: Interface>(self) -> T {
        T::new_unchecked(self)
    }

    pub fn downcast<T: Interface>(self) -> Option<T> {
        let (interface, _version) = self.backend.object_interface(self.id)?;
        if (self.client_side, interface.as_str()) == (T::CLIENT_SIDE, T::NAME) {
            Some(self.downcast_unchecked())
        } else {
            None
        }
    }
}
