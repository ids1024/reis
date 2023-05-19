use std::sync::{atomic::AtomicBool, Arc};

use crate::{Arg, Connection};

#[derive(Debug)]
struct ObjectInner {
    connection: Connection,
    id: u64,
    destroyed: AtomicBool,
}

#[derive(Clone, Debug)]
pub struct Object(Arc<ObjectInner>);

impl Object {
    pub fn new(connection: Connection, id: u64) -> Self {
        Self(Arc::new(ObjectInner {
            connection,
            id,
            destroyed: AtomicBool::new(false),
        }))
    }

    pub fn connection(&self) -> &Connection {
        &self.0.connection
    }

    pub fn id(&self) -> u64 {
        self.0.id
    }

    pub fn request(&self, opcode: u32, args: &[Arg]) -> rustix::io::Result<()> {
        self.connection().request(self.id(), opcode, args)
    }
}
