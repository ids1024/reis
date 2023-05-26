use std::sync::Arc;

use crate::{Arg, ConnectionInner, Interface};

#[derive(Clone, Debug)]
pub struct Object {
    connection: Arc<ConnectionInner>,
    id: u64,
}

impl Object {
    pub(crate) fn new(connection: Arc<ConnectionInner>, id: u64) -> Self {
        Self { connection, id }
    }

    pub fn connection(&self) -> &Arc<ConnectionInner> {
        &self.connection
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn request(&self, opcode: u32, args: &[Arg]) -> rustix::io::Result<()> {
        self.connection.request(self.id, opcode, args)
    }

    pub(crate) fn downcast_unchecked<T: Interface>(self) -> T {
        T::downcast_unchecked(self)
    }

    // XXX test ei vs ei
    pub fn downcast<T: Interface>(self) -> Option<T> {
        let (interface, _version) = self.connection.object_interface(self.id)?;
        if &interface == T::NAME {
            Some(T::downcast_unchecked(self))
        } else {
            None
        }
    }
}
