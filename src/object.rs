use std::{fmt, hash, sync::Arc};

use crate::{
    wire::{Arg, Backend, BackendWeak},
    Interface,
};

/// Representation of
/// [an object](https://libinput.pages.freedesktop.org/libei/doc/overview/index.html).
///
/// Contains all information required to send requests and events for the object.
#[derive(Clone)]
pub struct Object(Arc<ObjectInner>);

struct ObjectInner {
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
        self.0.id.hash(hasher);
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
            client_side,
            id,
            interface,
            version,
        }))
    }

    /// Returns a handle to the backend.
    ///
    /// Returns `None` if the backend has been destroyed.
    #[must_use]
    pub fn backend(&self) -> Option<Backend> {
        self.0.backend.upgrade()
    }

    /// Returns a weak handle to the backend that works even after the backend has been
    /// destroyed.
    pub(crate) fn backend_weak(&self) -> &BackendWeak {
        &self.0.backend
    }

    /// Returns `true` if the backend has this object, and `false` otherwise or if the backend
    /// has been destroyed.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        if let Some(backend) = self.backend() {
            backend.has_object_for_id(self.id())
        } else {
            false
        }
    }

    /// Returns the object's
    /// [ID](https://libinput.pages.freedesktop.org/libei/doc/types/index.html#object-ids).
    #[must_use]
    pub fn id(&self) -> u64 {
        self.0.id
    }

    /// Returns the tracked interface name, like `ei_device`.
    ///
    /// Interface names for new objects aren't usually transmitted, but rather come from
    /// the protocol definition.
    #[must_use]
    pub fn interface(&self) -> &str {
        &self.0.interface
    }

    /// Returns the version of the interface of this object.
    #[must_use]
    pub fn version(&self) -> u32 {
        self.0.version
    }

    /// Sends a request if running in a client or emits an event if running in a server.
    // TODO(axka, 2025-07-02): rename to "message" or "send"
    pub fn request(&self, opcode: u32, args: &[Arg]) {
        if let Some(backend) = self.backend() {
            backend.request(self.0.id, opcode, args);
        }
    }

    /// Returns an interface proxy without checking [`Object::interface`].
    pub(crate) fn downcast_unchecked<T: Interface>(self) -> T {
        T::new_unchecked(self)
    }

    /// Returns an `Arg` to reference this object in events or requests.
    pub(crate) fn as_arg(&self) -> Arg<'_> {
        Arg::Id(self.0.id)
    }

    /// Returns an interface proxy if it matches the tracked interface name and connection side
    /// (client or server).
    ///
    /// # Example
    ///
    /// Turning a generic object into a
    /// [client-side `ei_keyboard` proxy](crate::ei::keyboard::Keyboard).
    ///
    /// ```no_run
    /// use reis::{Object, ei::keyboard::{Keyboard, KeyState}};
    ///
    /// let object: Object;
    /// # object = todo!();
    ///
    /// assert_eq!(object.interface(), "ei_keyboard");
    /// let keyboard = object.downcast::<Keyboard>().unwrap();
    ///
    /// keyboard.key(0x41, KeyState::Press);
    /// ```
    // TODO(axka, 2025-07-02): return Result<T, Self>
    #[must_use]
    pub fn downcast<T: Interface>(self) -> Option<T> {
        if (self.0.client_side, self.interface()) == (T::CLIENT_SIDE, T::NAME) {
            Some(self.downcast_unchecked())
        } else {
            None
        }
    }
}
