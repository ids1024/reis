use std::sync::{atomic::AtomicBool, Arc};

use crate::{Arg, ConnectionInner, Interface};

#[derive(Debug)]
struct ObjectInner {
    connection: Arc<ConnectionInner>,
    id: u64,
    destroyed: AtomicBool,
}

#[derive(Clone, Debug)]
pub struct Object(Arc<ObjectInner>);

impl Object {
    pub(crate) fn new(connection: Arc<ConnectionInner>, id: u64) -> Self {
        Self(Arc::new(ObjectInner {
            connection,
            id,
            destroyed: AtomicBool::new(false),
        }))
    }

    pub fn connection(&self) -> &Arc<ConnectionInner> {
        &self.0.connection
    }

    pub fn id(&self) -> u64 {
        self.0.id
    }

    pub fn request(&self, opcode: u32, args: &[Arg]) -> rustix::io::Result<()> {
        self.connection().request(self.id(), opcode, args)
    }

    // XXX test ei vs ei
    pub fn downcast<T: Interface>(self) -> Option<T> {
        let (interface, _version) = self.connection().object_interface(self.id())?;
        if &interface == T::NAME {
            Some(T::downcast_unchecked(self))
        } else {
            None
        }
    }
}
