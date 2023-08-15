use std::{fmt, hash, sync::Arc};

use crate::{backend::BackendWeak, Arg, Backend, Interface};

#[derive(Clone)]
pub struct Object(Arc<ObjectInner>);

struct ObjectInner {
    // TODO use weak, like wayland-rs?
    backend: BackendWeak,
    client_side: bool,
    id: u64,
    interface: String,
    version: u32,
}

impl PartialEq for Object {
    fn eq(&self, rhs: &Self) -> bool {
        Arc::ptr_eq(&self.0, &rhs.0)
    }
}

impl Eq for Object {}

impl hash::Hash for Object {
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        self.0.id.hash(hasher)
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Object(_, {})", self.id())
    }
}

impl Object {
    // Should only be called by `Backend`
    // Other references are then cloned from the version stored there
    pub(crate) fn for_new_id(
        backend: BackendWeak,
        id: u64,
        client_side: bool,
        interface: String,
        version: u32,
    ) -> Self {
        Self(Arc::new(ObjectInner {
            backend,
            id,
            client_side,
            interface,
            version,
        }))
    }

    pub fn backend(&self) -> Option<Backend> {
        self.0.backend.upgrade()
    }

    pub(crate) fn backend_weak(&self) -> &BackendWeak {
        &self.0.backend
    }

    pub fn id(&self) -> u64 {
        self.0.id
    }

    pub fn interface(&self) -> &str {
        &self.0.interface
    }

    pub fn version(&self) -> u32 {
        self.0.version
    }

    pub fn request(&self, opcode: u32, args: &[Arg]) {
        if let Some(backend) = self.backend() {
            backend.request(self.0.id, opcode, args);
        }
    }

    pub(crate) fn downcast_unchecked<T: Interface>(self) -> T {
        T::new_unchecked(self)
    }

    pub(crate) fn as_arg(&self) -> crate::Arg<'_> {
        crate::Arg::Id(self.0.id)
    }

    pub fn downcast<T: Interface>(self) -> Option<T> {
        if (self.0.client_side, self.interface()) == (T::CLIENT_SIDE, T::NAME) {
            Some(self.downcast_unchecked())
        } else {
            None
        }
    }
}
