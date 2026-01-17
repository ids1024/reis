#![allow(
    unknown_lints,
    unused_imports,
    unused_parens,
    clippy::useless_conversion,
    clippy::double_parens,
    clippy::match_single_binding,
    clippy::unused_unit,
    clippy::empty_docs,
    clippy::doc_lazy_continuation,

    // Explicitly set to warn
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::semicolon_if_nothing_returned,
    clippy::used_underscore_binding,
    clippy::match_same_arms,
    clippy::str_to_string,
    missing_docs,
)]

// GENERATED FILE

use crate::wire;
/// Handshake object.
///
/// Server-side protocol definition module for interface `ei_handshake`.
///
/**
This is a special interface to setup the client as seen by the EIS
implementation. The object for this interface has the fixed object
id 0 and only exists until the connection has been set up, see the
ei_handshake.connection event.

The ei_handshake version is 1 until:
- the EIS implementation sends the handshake_version event with
  a version other than 1, and, in response,
- the client sends the handshake_version request with a
  version equal or lower to the EIS implementation version.

The EIS implementation must send the handshake_version event immediately
once the physical connection has been established.

Once the ei_connection.connection event has been sent the handshake
is destroyed by the EIS implementation.
 */
pub mod handshake {
    use crate::wire;

    /// Handshake object.
    ///
    /// Server-side interface proxy for interface `ei_handshake`.
    ///
    /**
    This is a special interface to setup the client as seen by the EIS
    implementation. The object for this interface has the fixed object
    id 0 and only exists until the connection has been set up, see the
    ei_handshake.connection event.

    The ei_handshake version is 1 until:
    - the EIS implementation sends the handshake_version event with
      a version other than 1, and, in response,
    - the client sends the handshake_version request with a
      version equal or lower to the EIS implementation version.

    The EIS implementation must send the handshake_version event immediately
    once the physical connection has been established.

    Once the ei_connection.connection event has been sent the handshake
    is destroyed by the EIS implementation.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Handshake(pub(crate) crate::Object);

    impl Handshake {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Handshake {}

    impl wire::Interface for Handshake {
        const NAME: &'static str = "ei_handshake";
        const VERSION: u32 = 1;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Handshake {}

    impl Handshake {
        /// Handshake version information from eis implementation.
        ///
        /// Informs the client that the EIS implementation supports the given
        /// version of the ei_handshake interface.
        ///
        /// This event is sent exactly once and immediately after connection to the
        /// EIS implementation.
        ///
        /// In response, the client must send the ei_handshake.handshake_version request
        /// with any version up to including the version provided in this event.
        /// See the ei_handshake.handshake_version request for details on what happens next.
        /// # Parameters
        ///
        /// - `version`: The interface version.
        ///
        pub fn handshake_version(&self, version: u32) -> () {
            let args = &[wire::Arg::Uint32(version.into())];

            self.0.request(0, args);

            ()
        }

        /// Interface support event.
        ///
        /// Informs the client that the EIS implementation supports the given named
        /// interface with the given maximum version number.
        ///
        /// The client must not assume those interfaces are supported unless
        /// and until those versions have been received.
        ///
        /// This request must not be sent for the "ei_handshake" interface, use
        /// the handshake_version event instead.
        ///
        /// This event may be sent by the EIS implementation for any
        /// other supported interface (but not necessarily all supported
        /// interfaces) before the ei_handshake.connection event.
        /// # Parameters
        ///
        /// - `name`: The interface name.
        /// - `version`: The interface version.
        ///
        pub fn interface_version(&self, name: &str, version: u32) -> () {
            let args = &[
                wire::Arg::String(name.into()),
                wire::Arg::Uint32(version.into()),
            ];

            self.0.request(1, args);

            ()
        }

        /// Provides the core connection object.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// Provides the client with the connection object that is the top-level
        /// object for all future requests and events.
        ///
        /// This event must be sent exactly once after the client sends the
        /// ei_handshake.finish request to the EIS implementation.
        ///
        /// The ei_handshake object will be destroyed by the EIS implementation
        /// immediately after this event has been sent, the client must not attempt
        /// to use it after that point.
        ///
        /// The version sent by the EIS implementation is the version of the `ei_connection`
        /// interface as announced by ei_handshake.interface_version, or any
        /// lower version.
        ///
        /// The serial number is the start value of the EIS implementation's serial
        /// number sequence. Clients must not assume any specific value for this
        /// serial number. Any future serial number in any event is monotonically
        /// increasing by an unspecified amount.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        /// - `version`: The version of the connection object.
        ///
        pub fn connection(&self, serial: u32, version: u32) -> (super::connection::Connection) {
            let connection = self
                .0
                .backend_weak()
                .new_object("ei_connection".to_string(), version);
            let args = &[
                wire::Arg::Uint32(serial.into()),
                wire::Arg::NewId(connection.id().into()),
                wire::Arg::Uint32(version.into()),
            ];

            self.0.request(2, args);
            self.0.backend_weak().remove_id(self.0.id());

            (super::connection::Connection(connection))
        }
    }

    pub use crate::eiproto_enum::handshake::ContextType;

    /// All requests of interface `ei_handshake`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Handshake version information from ei client.
        ///
        /// Informs the EIS implementation that this client supports the given
        /// version of the ei_handshake interface. The version number must be less
        /// than or equal to the version in the handshake_version event sent by the
        /// EIS implementation when the connection was established.
        ///
        /// Immediately after sending this request, the client must assume the negotiated
        /// version number for the ei_handshake interface and the EIS implementation
        /// may send events and process requests matching that version.
        ///
        /// This request must be sent exactly once and it must be the first request
        /// the client sends.
        HandshakeVersion {
            /// The interface version.
            version: u32,
        },
        /// Setup completion request.
        ///
        /// Informs the EIS implementation that configuration is complete.
        ///
        /// In the future (and possibly after requiring user interaction),
        /// the EIS implementation responds by sending the ei_handshake.connection event.
        Finish,
        /// Context type information.
        ///
        /// Informs the EIS implementation of the type of this context. The context
        /// type defines whether the EI client will send input events to the EIS
        /// implementation or receive input events from it.
        ///
        /// Depending on the context type, certain requests must not be used and some
        /// events must not be sent by the EIS implementation.
        ///
        /// This request is optional, the default client type is context_type.receiver.
        /// This request must not be sent more than once and must be sent before
        /// ei_handshake.finish.
        ContextType {
            /// The connection's context type.
            context_type: ContextType,
        },
        /// Client name.
        ///
        /// Informs the EIS implementation of the client name. The name is a
        /// human-presentable UTF-8 string and should represent the client name as
        /// accurately as possible. This name may be presented to the user for
        /// identification of this client (e.g. to confirm the client has
        /// permissions to connect).
        ///
        /// There is no requirement for the EIS implementation to use this name. For
        /// example, where the client is managed through an XDG Desktop Portal an EIS
        /// implementation would typically use client identification information sent
        /// by the portal instead.
        ///
        /// This request is optional, the default client name is implementation-defined.
        /// This request must not be sent more than once and must be sent before
        /// ei_handshake.finish.
        Name {
            /// The client name.
            name: String,
        },
        /// Interface support information.
        ///
        /// Informs the EIS implementation that the EI client supports the given
        /// named interface with the given maximum version number.
        ///
        /// Future objects created by the EIS implementation will
        /// use the respective interface version (or any lesser version)
        /// as announced by the ei_connection.interface_version event.
        ///
        /// This request must be sent for the "ei_connection" interface,
        /// failing to do so will result in the EIS implementation disconnecting
        /// the client on ei_handshake.finish.
        ///
        /// This request must not be sent for the "ei_handshake" interface, use
        /// the ei_handshake.handshake_version request instead.
        ///
        /// Note that an EIS implementation may consider some interfaces to
        /// be required and immediately ei_connection.disconnect a client
        /// not supporting those interfaces.
        ///
        /// This request must not be sent more than once per interface and must be
        /// sent before ei_handshake.finish.
        InterfaceVersion {
            /// The interface name.
            name: String,
            /// The interface version.
            version: u32,
        },
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("handshake_version"),
                1 => Some("finish"),
                2 => Some("context_type"),
                3 => Some("name"),
                4 => Some("interface_version"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => {
                    let version = _bytes.read_arg()?;

                    Ok(Self::HandshakeVersion { version })
                }
                1 => Ok(Self::Finish),
                2 => {
                    let context_type = _bytes.read_arg()?;

                    Ok(Self::ContextType { context_type })
                }
                3 => {
                    let name = _bytes.read_arg()?;

                    Ok(Self::Name { name })
                }
                4 => {
                    let name = _bytes.read_arg()?;
                    let version = _bytes.read_arg()?;

                    Ok(Self::InterfaceVersion { name, version })
                }
                opcode => Err(wire::ParseError::InvalidOpcode("handshake", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::HandshakeVersion { version } => {
                    args.push(version.as_arg());
                }
                Self::Finish => {}
                Self::ContextType { context_type } => {
                    args.push(context_type.as_arg());
                }
                Self::Name { name } => {
                    args.push(name.as_arg());
                }
                Self::InterfaceVersion { name, version } => {
                    args.push(name.as_arg());
                    args.push(version.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use handshake::Handshake;

/// Core connection object.
///
/// Server-side protocol definition module for interface `ei_connection`.
///
/**
The core connection object. This is the top-level object for any communication
with the EIS implementation.

Note that for a client to receive this object, it must announce
support for this interface in ei_handshake.interface_version.
 */
pub mod connection {
    use crate::wire;

    /// Core connection object.
    ///
    /// Server-side interface proxy for interface `ei_connection`.
    ///
    /**
    The core connection object. This is the top-level object for any communication
    with the EIS implementation.

    Note that for a client to receive this object, it must announce
    support for this interface in ei_handshake.interface_version.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Connection(pub(crate) crate::Object);

    impl Connection {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Connection {}

    impl wire::Interface for Connection {
        const NAME: &'static str = "ei_connection";
        const VERSION: u32 = 1;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Connection {}

    impl Connection {
        /// Disconnection event.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This event may be sent by the EIS implementation immediately before
        /// the client is disconnected. The last_serial argument is set to the last
        /// serial number used in an event by the EIS implementation.
        ///
        /// Where a client is disconnected by EIS on purpose, for example after
        /// a user interaction, the reason is disconnect_reason.disconnected (i.e. zero)
        /// and the explanation is NULL.
        ///
        /// Where a client is disconnected due to some invalid request or other
        /// protocol error, the reason is one of disconnect_reason (i.e. nonzero) and
        /// explanation may contain a string explaining why. This string is
        /// intended to help debugging only and is not guaranteed to stay constant.
        ///
        /// The ei_connection object will be destroyed by the
        /// EIS implementation immediately after this event has been sent, a
        /// client must not attempt to use it after that point.
        ///
        /// There is no guarantee this event is sent - the connection may be closed
        /// without a disconnection event.
        /// # Parameters
        ///
        /// - `last_serial`: The last serial sent by the eis implementation.
        /// - `reason`: The reason for being disconnected.
        /// - `explanation`: An explanation for debugging purposes.
        ///
        pub fn disconnected(
            &self,
            last_serial: u32,
            reason: DisconnectReason,
            explanation: Option<&str>,
        ) -> () {
            let args = &[
                wire::Arg::Uint32(last_serial.into()),
                wire::Arg::Uint32(reason.into()),
                wire::Arg::String(explanation.into()),
            ];

            self.0.request(0, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }

        /// Seat presence information.
        ///
        /// Informs the client that a new seat has been added.
        ///
        /// A seat is a set of input devices that logically belong together.
        ///
        /// This event is only sent if the client announced support for the
        /// "ei_seat" interface in ei_handshake.interface_version.
        /// The interface version is equal or less to the client-supported
        /// version in ei_handshake.interface_version for the "ei_seat"
        /// interface.
        /// # Parameters
        ///
        /// - `version`: The interface version.
        ///
        pub fn seat(&self, version: u32) -> (super::seat::Seat) {
            let seat = self
                .0
                .backend_weak()
                .new_object("ei_seat".to_string(), version);
            let args = &[
                wire::Arg::NewId(seat.id().into()),
                wire::Arg::Uint32(version.into()),
            ];

            self.0.request(1, args);

            (super::seat::Seat(seat))
        }

        /// Invalid object in request notification.
        ///
        /// Informs the client that an object ID used in an earlier request was
        /// invalid and does not exist.
        ///
        /// This event is sent by the EIS implementation when an object that
        /// does not exist as seen by the EIS implementation. The protocol is
        /// asynchronous and this may occur e.g. when the EIS implementation
        /// destroys an object at the same time as the client requests functionality
        /// from that object. For example, an EIS implementation may send
        /// ei_device.destroyed and destroy the device's resources (and protocol object)
        /// at the same time as the client attempts to ei_device.start_emulating
        /// on that object.
        ///
        /// It is the client's responsibility to unwind any state changes done
        /// to the object since the last successful message.
        /// # Parameters
        ///
        /// - `last_serial`: The last serial sent by the eis implementation.
        /// - `invalid_id`
        ///
        pub fn invalid_object(&self, last_serial: u32, invalid_id: u64) -> () {
            let args = &[
                wire::Arg::Uint32(last_serial.into()),
                wire::Arg::Uint64(invalid_id.into()),
            ];

            self.0.request(2, args);

            ()
        }

        /// Ping event.
        ///
        /// The ping event asks the client to emit the 'done' event
        /// on the provided ei_pingpong object. Since requests are
        /// handled in-order and events are delivered in-order, this can
        /// be used as a synchronization point to ensure all previous requests
        /// and the resulting events have been handled.
        ///
        /// The object returned by this request must be destroyed by the
        /// ei client implementation after the callback is fired and as
        /// such the client must not attempt to use it after that point.
        ///
        /// The callback_data in the resulting ei_pingpong.done request is
        /// ignored by the EIS implementation.
        ///
        /// Note that for a EIS implementation to use this request the client must
        /// announce support for this interface in ei_handshake.interface_version. It is
        /// a protocol violation to send this event to a client without the
        /// "ei_pingpong" interface.
        /// # Parameters
        ///
        /// - `version`: The version of the callback object.
        ///
        pub fn ping(&self, version: u32) -> (super::pingpong::Pingpong) {
            let ping = self
                .0
                .backend_weak()
                .new_object("ei_pingpong".to_string(), version);
            let args = &[
                wire::Arg::NewId(ping.id().into()),
                wire::Arg::Uint32(version.into()),
            ];

            self.0.request(3, args);

            (super::pingpong::Pingpong(ping))
        }
    }

    pub use crate::eiproto_enum::connection::DisconnectReason;

    /// All requests of interface `ei_connection`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Asynchronous roundtrip.
        ///
        /// Requests the EIS implementation to emit the ei_callback.done event on
        /// the returned ei_callback object. Since requests are handled in-order
        /// and events are delivered in-order, this can be used as a
        /// synchronization point to ensure all previous requests and the resulting
        /// events have been handled.
        ///
        /// The object returned by this request will be destroyed by the
        /// EIS implementation after the callback is fired and as such the client must not
        /// attempt to use it after that point.
        ///
        /// The callback_data in the ei_callback.done event must be zero.
        ///
        /// Note that for a client to use this request it must announce
        /// support for the `ei_callback` interface in ei_handshake.interface_version.
        /// It is a protocol violation to request sync without having announced the
        /// `ei_callback` interface and the EIS implementation must disconnect
        /// the client.
        Sync {
            /// Callback object for the sync request.
            callback: super::callback::Callback,
        },
        /// Disconnection request.
        ///
        /// **Note:** This request is a destructor.
        ///
        /// A request to the EIS implementation that this client should be disconnected.
        /// This is a courtesy request to allow the EIS implementation to distinguish
        /// between a client disconnecting on purpose and one disconnecting through the
        /// socket becoming invalid.
        ///
        /// Immediately after sending this request, the client may destroy the
        /// ei_connection object and it should close the socket. The EIS implementation
        /// will treat the connection as already disconnected on receipt and does not
        /// send the ei_connection.disconnect event in response to this request.
        Disconnect,
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("sync"),
                1 => Some("disconnect"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => {
                    let callback = _bytes.read_arg()?;
                    let version = _bytes.read_arg()?;

                    Ok(Self::Sync {
                        callback: _bytes.backend().new_peer_interface(callback, version)?,
                    })
                }
                1 => Ok(Self::Disconnect),
                opcode => Err(wire::ParseError::InvalidOpcode("connection", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::Sync { callback } => {
                    args.push(callback.as_arg());
                }
                Self::Disconnect => {}
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use connection::Connection;

/// Callback object.
///
/// Server-side protocol definition module for interface `ei_callback`.
///
/**
Interface for ensuring a roundtrip to the EIS implementation.
Clients can handle the 'done' event to get notified when
the related request that created the ei_callback object is done.
 */
pub mod callback {
    use crate::wire;

    /// Callback object.
    ///
    /// Server-side interface proxy for interface `ei_callback`.
    ///
    /**
    Interface for ensuring a roundtrip to the EIS implementation.
    Clients can handle the 'done' event to get notified when
    the related request that created the ei_callback object is done.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Callback(pub(crate) crate::Object);

    impl Callback {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Callback {}

    impl wire::Interface for Callback {
        const NAME: &'static str = "ei_callback";
        const VERSION: u32 = 1;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Callback {}

    impl Callback {
        /// Done event.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// Informs the client that the associated request is finished. The EIS
        /// implementation must destroy the ei_callback object immediately after
        /// sending this event this event and as such the client must not attempt to
        /// use it after that point.
        /// # Parameters
        ///
        /// - `callback_data`: Request-specific data for the callback.
        ///
        pub fn done(&self, callback_data: u64) -> () {
            let args = &[wire::Arg::Uint64(callback_data.into())];

            self.0.request(0, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }
    }

    /// All requests of interface `ei_callback`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {}

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                opcode => Err(wire::ParseError::InvalidOpcode("callback", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use callback::Callback;

/// Callback object.
///
/// Server-side protocol definition module for interface `ei_pingpong`.
///
/**
Interface for ensuring a roundtrip to the client implementation.
This interface is identical to ei_callback but is intended for
the EIS implementation to enforce a roundtrip to the client.
 */
pub mod pingpong {
    use crate::wire;

    /// Callback object.
    ///
    /// Server-side interface proxy for interface `ei_pingpong`.
    ///
    /**
    Interface for ensuring a roundtrip to the client implementation.
    This interface is identical to ei_callback but is intended for
    the EIS implementation to enforce a roundtrip to the client.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Pingpong(pub(crate) crate::Object);

    impl Pingpong {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Pingpong {}

    impl wire::Interface for Pingpong {
        const NAME: &'static str = "ei_pingpong";
        const VERSION: u32 = 1;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Pingpong {}

    impl Pingpong {}

    /// All requests of interface `ei_pingpong`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Done event.
        ///
        /// **Note:** This request is a destructor.
        ///
        /// Informs the EIS implementation when the associated event is finished.
        /// The client must destroy the ei_pingpong object immediately after this
        /// request and as such the server must not attempt to use it after that
        /// point.
        Done {
            /// Request-specific data for the callback.
            callback_data: u64,
        },
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("done"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => {
                    let callback_data = _bytes.read_arg()?;

                    Ok(Self::Done { callback_data })
                }
                opcode => Err(wire::ParseError::InvalidOpcode("pingpong", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::Done { callback_data } => {
                    args.push(callback_data.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use pingpong::Pingpong;

/// Set of input devices that logically belong together.
///
/// Server-side protocol definition module for interface `ei_seat`.
///
/**
An ei_seat represents a set of input devices that logically belong together. In most
cases only one seat is present and all input devices on that seat share the same
pointer and keyboard focus.

A seat has potential capabilities, a client is expected to bind to those capabilities.
The EIS implementation then creates logical input devices based on the capabilities the
client is interested in.

Immediately after creation of the ei_seat object, the EIS implementation sends a burst
of events with information about this seat. This burst of events is terminated by the
ei_seat.done event.
 */
pub mod seat {
    use crate::wire;

    /// Set of input devices that logically belong together.
    ///
    /// Server-side interface proxy for interface `ei_seat`.
    ///
    /**
    An ei_seat represents a set of input devices that logically belong together. In most
    cases only one seat is present and all input devices on that seat share the same
    pointer and keyboard focus.

    A seat has potential capabilities, a client is expected to bind to those capabilities.
    The EIS implementation then creates logical input devices based on the capabilities the
    client is interested in.

    Immediately after creation of the ei_seat object, the EIS implementation sends a burst
    of events with information about this seat. This burst of events is terminated by the
    ei_seat.done event.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Seat(pub(crate) crate::Object);

    impl Seat {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Seat {}

    impl wire::Interface for Seat {
        const NAME: &'static str = "ei_seat";
        const VERSION: u32 = 1;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Seat {}

    impl Seat {
        /// Seat removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// Informs the client that this seat has been removed, and that it should
        /// release all associated resources.
        ///
        /// This ei_seat object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn destroyed(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(0, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }

        /// Seat name notification.
        ///
        /// The name of this seat, if any. This event is optional and sent once immediately
        /// after object creation.
        ///
        /// It is a protocol violation to send this event after the ei_seat.done event.
        /// # Parameters
        ///
        /// - `name`: The seat name.
        ///
        pub fn name(&self, name: &str) -> () {
            let args = &[wire::Arg::String(name.into())];

            self.0.request(1, args);

            ()
        }

        /// Seat capability notification.
        ///
        /// Informs the client that this seat supports devices with the given
        /// interface. The interface must be mapped to a bitmask by the EIS
        /// implementation. The client may apply the binary OR operation onto these
        /// bitmasks in ei_seat.bind. In response, the EIS implementation may then
        /// create devices based on those bound capabilities.
        ///
        /// For example, an EIS implementation may advertise support for
        /// `ei_pointer` devices at bitmask `0x1`, `ei_keyboard` devices at `0x4`
        /// and `ei_touchscreen` devices at `0x8`. A client may then execute the
        /// request `ei_seat.bind(0xC)` to bind to keyboard and touchscreen devices
        /// but not pointing devices.
        ///
        /// The EIS implementation must not advertise capabilities for interfaces
        /// that have not been negotiated in the ei_handshake object.
        ///
        /// The EIS implementation may decide which capabilities a given seat has.
        /// After ei_seat.done, the capabilities are constant for the lifetime of
        /// the seat but may differ between seats. The masks may be sparse bitwise.
        ///
        /// This event is sent multiple time for each supported interface, finishing
        /// with ei_seat.done.
        /// # Parameters
        ///
        /// - `mask`: The mask representing this capability.
        /// - `interface`: The interface name for this capability.
        ///
        pub fn capability(&self, mask: u64, interface: &str) -> () {
            let args = &[
                wire::Arg::Uint64(mask.into()),
                wire::Arg::String(interface.into()),
            ];

            self.0.request(2, args);

            ()
        }

        /// Seat setup completion notification.
        ///
        /// Notification that the initial burst of events is complete and
        /// the client can set up this seat now.
        ///
        /// It is a protocol violation to send this event more than once.
        pub fn done(&self) -> () {
            let args = &[];

            self.0.request(3, args);

            ()
        }

        /// Device presence notification.
        ///
        /// Informs the client that a new device has been added to the seat.
        ///
        /// The EIS implementation must never announce devices that have not been bound to with ei_seat.bind.
        ///
        /// This event is only sent if the client announced support for the
        /// `ei_device` interface in ei_handshake.interface_version. The interface
        /// version is less than or equal to the client-supported version in
        /// ei_handshake.interface_version for the `ei_device` interface.
        /// # Parameters
        ///
        /// - `version`: The interface version.
        ///
        pub fn device(&self, version: u32) -> (super::device::Device) {
            let device = self
                .0
                .backend_weak()
                .new_object("ei_device".to_string(), version);
            let args = &[
                wire::Arg::NewId(device.id().into()),
                wire::Arg::Uint32(version.into()),
            ];

            self.0.request(4, args);

            (super::device::Device(device))
        }
    }

    /// All requests of interface `ei_seat`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Seat removal request.
        ///
        /// Informs the EIS implementation that the client is no longer interested
        /// in this seat. The EIS implementation should release any resources
        /// associated with this seat and send the ei_seat.destroyed event once
        /// finished.
        ///
        /// Note that releasing a seat does not guarantee that another seat becomes
        /// available. In other words, in most single-seat cases, releasing the seat
        /// means that the connection becomes effectively inert.
        Release,
        /// Seat binding.
        ///
        /// Binds to the given bitmask of capabilities. Each one of the bit values
        /// in the given bitmask must originate from one of the ei_seat.capability
        /// events. See its documentation for more examples.
        ///
        /// The EIS implementation should return compatible devices with
        /// ei_seat.device events.
        ///
        /// Binding masks that are not supported in the ei_device's interface
        /// version is a client bug and may result in disconnection.
        ///
        /// A client may send this request multiple times to adjust the capabilities
        /// it is interested in. If previously-bound capabilities are dropped by the
        /// client, the EIS implementation may ei_device.remove devices that have
        /// these capabilities.
        Bind {
            /// Bitmask of the capabilities.
            capabilities: u64,
        },
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("release"),
                1 => Some("bind"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => Ok(Self::Release),
                1 => {
                    let capabilities = _bytes.read_arg()?;

                    Ok(Self::Bind { capabilities })
                }
                opcode => Err(wire::ParseError::InvalidOpcode("seat", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::Release => {}
                Self::Bind { capabilities } => {
                    args.push(capabilities.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use seat::Seat;

/// Logical input device.
///
/// Server-side protocol definition module for interface `ei_device`.
///
/**
An ei_device represents a single logical input device. Like physical input
devices an ei_device may have multiple capabilities and may e.g. function
as pointer and keyboard.

Depending on the ei_handshake.context_type, an ei_device can
emulate events via client requests or receive events. It is a protocol violation
to emulate certain events on a receiver device, or for the EIS implementation
to send certain events to the device. See the individual request/event documentation
for details.
 */
pub mod device {
    use crate::wire;

    /// Logical input device.
    ///
    /// Server-side interface proxy for interface `ei_device`.
    ///
    /**
    An ei_device represents a single logical input device. Like physical input
    devices an ei_device may have multiple capabilities and may e.g. function
    as pointer and keyboard.

    Depending on the ei_handshake.context_type, an ei_device can
    emulate events via client requests or receive events. It is a protocol violation
    to emulate certain events on a receiver device, or for the EIS implementation
    to send certain events to the device. See the individual request/event documentation
    for details.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Device(pub(crate) crate::Object);

    impl Device {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Device {}

    impl wire::Interface for Device {
        const NAME: &'static str = "ei_device";
        const VERSION: u32 = 2;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Device {}

    impl Device {
        /// Device removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This device has been removed and a client should release all
        /// associated resources.
        ///
        /// This ei_device object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn destroyed(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(0, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }

        /// Device name notification.
        ///
        /// The name of this device, if any. This event is optional and sent once immediately
        /// after object creation.
        ///
        /// It is a protocol violation to send this event after the ei_device.done event.
        /// # Parameters
        ///
        /// - `name`: The device name.
        ///
        pub fn name(&self, name: &str) -> () {
            let args = &[wire::Arg::String(name.into())];

            self.0.request(1, args);

            ()
        }

        /// Device type notification.
        ///
        /// The device type, one of virtual or physical.
        ///
        /// Devices of type ei_device.device_type.physical are only supported for
        /// clients of type ei_handshake.context_type.receiver.
        ///
        /// This event is sent once immediately after object creation.
        /// It is a protocol violation to send this event after the ei_device.done event.
        /// # Parameters
        ///
        /// - `device_type`: The device type.
        ///
        pub fn device_type(&self, device_type: DeviceType) -> () {
            let args = &[wire::Arg::Uint32(device_type.into())];

            self.0.request(2, args);

            ()
        }

        /// Device dimensions notification.
        ///
        /// The device dimensions in mm. This event is optional and sent once immediately
        /// after object creation.
        ///
        /// This event is only sent for devices of ei_device.device_type.physical.
        ///
        /// It is a protocol violation to send this event after the ei_device.done event.
        /// # Parameters
        ///
        /// - `width`: The device physical width in mm.
        /// - `height`: The device physical height in mm.
        ///
        pub fn dimensions(&self, width: u32, height: u32) -> () {
            let args = &[
                wire::Arg::Uint32(width.into()),
                wire::Arg::Uint32(height.into()),
            ];

            self.0.request(3, args);

            ()
        }

        /// Device region notification.
        ///
        /// Notifies the client of one region. The number of regions is constant for a device
        /// and all regions are announced immediately after object creation.
        ///
        /// A region is rectangular and defined by an x/y offset and a width and a height.
        /// A region defines the area on an EIS desktop layout that is accessible by
        /// this device - this region may not be the full area of the desktop.
        /// Input events may only be sent for points within the regions.
        ///
        /// The use of regions is private to the EIS compositor and coordinates may not
        /// match the size of the actual desktop. For example, a compositor may set a
        /// 1920x1080 region to represent a 4K monitor and transparently map input
        /// events into the respective true pixels.
        ///
        /// Absolute devices may have different regions, it is up to the client to
        /// send events through the correct device to target the right pixel. For
        /// example, a dual-head setup my have two absolute devices, the first with
        /// a zero offset region spanning the left screen, the second with a nonzero
        /// offset spanning the right screen.
        ///
        /// The physical scale denotes a constant multiplication factor that needs to be applied to
        /// any relative movement on this region for that movement to match the same
        /// *physical* movement on another region.
        ///
        /// It is an EIS implementation bug to advertise the touch and/or absolute pointer capability
        /// on a device_type.virtual device without advertising an ei_region for this device.
        ///
        /// This event is optional and sent immediately after object creation. Where a device
        /// has multiple regions, this event is sent once for each region.
        /// It is a protocol violation to send this event after the ei_device.done event.
        ///
        /// Note: the fourth argument ('hight') was misspelled when the protocol was declared
        /// stable but changing the name is an API breaking change.
        /// # Parameters
        ///
        /// - `offset_x`: Region x offset in logical pixels.
        /// - `offset_y`: Region y offset in logical pixels.
        /// - `width`: Region width in logical pixels.
        /// - `hight`: Region height in logical pixels.
        /// - `scale`: The physical scale for this region.
        ///
        pub fn region(
            &self,
            offset_x: u32,
            offset_y: u32,
            width: u32,
            hight: u32,
            scale: f32,
        ) -> () {
            let args = &[
                wire::Arg::Uint32(offset_x.into()),
                wire::Arg::Uint32(offset_y.into()),
                wire::Arg::Uint32(width.into()),
                wire::Arg::Uint32(hight.into()),
                wire::Arg::Float(scale.into()),
            ];

            self.0.request(4, args);

            ()
        }

        /// Device capability notification.
        ///
        /// Notification that a new device has a sub-interface.
        ///
        /// This event may be sent for the following interfaces:
        /// - "ei_pointer"
        /// - "ei_pointer_absolute"
        /// - "ei_scroll"
        /// - "ei_button"
        /// - "ei_keyboard"
        /// - "ei_touchscreen"
        /// The interface version is equal or less to the client-supported
        /// version in ei_handshake.interface_version for the respective interface.
        ///
        /// It is a protocol violation to send a notification for an interface that
        /// the client has not bound to with ei_seat.bind.
        ///
        /// This event is optional and sent immediately after object creation
        /// and at most once per interface.
        /// It is a protocol violation to send this event after the ei_device.done event.
        /// # Parameters
        ///
        /// - `version`: The interface version.
        ///
        pub fn interface<InterfaceName: crate::eis::Interface>(
            &self,
            version: u32,
        ) -> (InterfaceName) {
            let object = self
                .0
                .backend_weak()
                .new_object(InterfaceName::NAME.to_string(), version);
            let args = &[
                wire::Arg::NewId(object.id().into()),
                wire::Arg::String(Some(InterfaceName::NAME)),
                wire::Arg::Uint32(version.into()),
            ];

            self.0.request(5, args);

            (object.downcast_unchecked())
        }

        /// Device setup completion notification.
        ///
        /// Notification that the initial burst of events is complete and
        /// the client can set up this device now.
        ///
        /// It is a protocol violation to send this event more than once per device.
        pub fn done(&self) -> () {
            let args = &[];

            self.0.request(6, args);

            ()
        }

        /// Device resumed notification.
        ///
        /// Notification that the device has been resumed by the EIS implementation
        /// and (depending on the ei_handshake.context_type) the client may request
        /// ei_device.start_emulating or the EIS implementation may
        /// ei_device.start_emulating events.
        ///
        /// It is a client bug to request emulation of events on a device that is
        /// not resumed. The EIS implementation may silently discard such events.
        ///
        /// A newly advertised device is in the ei_device.paused state.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn resumed(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(7, args);

            ()
        }

        /// Device paused notification.
        ///
        /// Notification that the device has been paused by the EIS implementation
        /// and no further events will be accepted on this device until
        /// it is resumed again.
        ///
        /// For devices of ei_device_setup.context_type sender, the client thus does
        /// not need to request ei_device.stop_emulating and may request
        /// ei_device.start_emulating after a subsequent ei_device.resumed.
        ///
        /// For devices of ei_device_setup.context_type receiver and where
        /// the EIS implementation did not send a ei_device.stop_emulating
        /// prior to this event, the device may send a ei_device.start_emulating
        /// event after a subsequent ei_device.resumed event.
        ///
        /// Pausing a device resets the logical state of the device to neutral.
        /// This includes:
        /// - any buttons or keys logically down are released
        /// - any modifiers logically down are released
        /// - any touches logically down are released
        ///
        /// It is a client bug to request emulation of events on a device that is
        /// not resumed. The EIS implementation may silently discard such events.
        ///
        /// A newly advertised device is in the ei_device.paused state.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn paused(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(8, args);

            ()
        }

        /// Device start emulating event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_device.start_emulating request for details.
        ///
        /// It is a protocol violation to send this event for a client
        /// of an ei_handshake.context_type other than receiver.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        /// - `sequence`
        ///
        pub fn start_emulating(&self, serial: u32, sequence: u32) -> () {
            let args = &[
                wire::Arg::Uint32(serial.into()),
                wire::Arg::Uint32(sequence.into()),
            ];

            self.0.request(9, args);

            ()
        }

        /// Device stop emulating event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_device.stop_emulating request for details.
        ///
        /// It is a protocol violation to send this event for a client
        /// of an ei_handshake.context_type other than receiver.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn stop_emulating(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(10, args);

            ()
        }

        /// Device frame event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_device.frame request for details.
        ///
        /// It is a protocol violation to send this event for a client
        /// of an ei_handshake.context_type other than receiver.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        /// - `timestamp`: Timestamp in microseconds.
        ///
        pub fn frame(&self, serial: u32, timestamp: u64) -> () {
            let args = &[
                wire::Arg::Uint32(serial.into()),
                wire::Arg::Uint64(timestamp.into()),
            ];

            self.0.request(11, args);

            ()
        }

        /// Region id notification.
        ///
        /// Notifies the client that the region specified in the next ei_device.region
        /// event is to be assigned the given mapping_id.
        ///
        /// This ID can be used by the client to identify an external resource that has a
        /// relationship with this region.
        /// For example the client may receive a data stream with the video
        /// data that this region represents. By attaching the same identifier to the data
        /// stream and this region the EIS implementation can inform the client
        /// that the video data stream and the region represent paired data.
        ///
        /// This event is optional and sent immediately after object creation but before
        /// the corresponding ei_device.region event. Where a device has multiple regions,
        /// this event may be sent zero or one time for each region.
        /// It is a protocol violation to send this event after the ei_device.done event or
        /// to send this event without a corresponding following ei_device.region event.
        /// # Parameters
        ///
        /// - `mapping_id`: Region mapping id.
        ///
        pub fn region_mapping_id(&self, mapping_id: &str) -> () {
            let args = &[wire::Arg::String(mapping_id.into())];

            self.0.request(12, args);

            ()
        }
    }

    pub use crate::eiproto_enum::device::DeviceType;

    /// All requests of interface `ei_device`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Device removal request.
        ///
        /// Notification that the client is no longer interested in this device.
        ///
        /// Note that releasing a device does not guarantee another device becomes available.
        ///
        /// The EIS implementation will release any resources related to this device and
        /// send the ei_device.destroyed event once complete.
        Release,
        /// Device start emulating request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Notify the EIS implementation that the given device is about to start
        /// sending events. This should be seen more as a transactional boundary than a
        /// time-based boundary. The primary use-cases for this are to allow for setup on
        /// the EIS implementation side and/or UI updates to indicate that a device is
        /// sending events now and for out-of-band information to sync with a given event
        /// sequence.
        ///
        /// There is no actual requirement that events start immediately once emulation
        /// starts and there is no requirement that a client calls ei_device.stop_emulating
        /// after the most recent events.
        /// For example, in a remote desktop use-case the client would call
        /// ei_device.start_emulating once the remote desktop session starts (rather than when
        /// the device sends events) and ei_device.stop_emulating once the remote desktop
        /// session stops.
        ///
        /// The sequence number identifies this transaction between start/stop emulating.
        /// It must go up by at least 1 on each call to ei_device.start_emulating.
        /// Wraparound must be handled by the EIS implementation but callers must ensure
        /// that detection of wraparound is possible.
        ///
        /// It is a protocol violation to request ei_device.start_emulating after
        /// ei_device.start_emulating without an intermediate stop_emulating.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than sender.
        StartEmulating {
            /// The last serial sent by the eis implementation.
            last_serial: u32,
            /// Sequence number to identify this emulation sequence.
            sequence: u32,
        },
        /// Device start emulating request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Notify the EIS implementation that the given device is no longer sending
        /// events. See ei_device.start_emulating for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than sender.
        StopEmulating {
            /// The last serial sent by the eis implementation.
            last_serial: u32,
        },
        /// Device frame request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Generate a frame event to group the current set of events
        /// into a logical hardware event. This function must be called after one
        /// or more events on any of ei_pointer, ei_pointer_absolute,
        /// ei_scroll, ei_button, ei_keyboard or ei_touchscreen has
        /// been requested by the EIS implementation.
        ///
        /// The EIS implementation should not process changes to the device state
        /// until the ei_device.frame event. For example, pressing and releasing
        /// a key within the same frame is a logical noop.
        ///
        /// The given timestamp applies to all events in the current frame.
        /// The timestamp must be in microseconds of CLOCK_MONOTONIC.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than sender.
        Frame {
            /// The last serial sent by the eis implementation.
            last_serial: u32,
            /// Timestamp in microseconds.
            timestamp: u64,
        },
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("release"),
                1 => Some("start_emulating"),
                2 => Some("stop_emulating"),
                3 => Some("frame"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => Ok(Self::Release),
                1 => {
                    let last_serial = _bytes.read_arg()?;
                    let sequence = _bytes.read_arg()?;

                    Ok(Self::StartEmulating {
                        last_serial,
                        sequence,
                    })
                }
                2 => {
                    let last_serial = _bytes.read_arg()?;

                    Ok(Self::StopEmulating { last_serial })
                }
                3 => {
                    let last_serial = _bytes.read_arg()?;
                    let timestamp = _bytes.read_arg()?;

                    Ok(Self::Frame {
                        last_serial,
                        timestamp,
                    })
                }
                opcode => Err(wire::ParseError::InvalidOpcode("device", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::Release => {}
                Self::StartEmulating {
                    last_serial,
                    sequence,
                } => {
                    args.push(last_serial.as_arg());
                    args.push(sequence.as_arg());
                }
                Self::StopEmulating { last_serial } => {
                    args.push(last_serial.as_arg());
                }
                Self::Frame {
                    last_serial,
                    timestamp,
                } => {
                    args.push(last_serial.as_arg());
                    args.push(timestamp.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use device::Device;

/// Device sub-interface for relative pointer motion.
///
/// Server-side protocol definition module for interface `ei_pointer`.
///
/**
Interface for relative pointer motion requests and events.

This interface is only provided once per device and where a client
requests ei_pointer.release the interface does not get re-initialized. An
EIS implementation may adjust the behavior of the device (including removing
the device) if the interface is released.
 */
pub mod pointer {
    use crate::wire;

    /// Device sub-interface for relative pointer motion.
    ///
    /// Server-side interface proxy for interface `ei_pointer`.
    ///
    /**
    Interface for relative pointer motion requests and events.

    This interface is only provided once per device and where a client
    requests ei_pointer.release the interface does not get re-initialized. An
    EIS implementation may adjust the behavior of the device (including removing
    the device) if the interface is released.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Pointer(pub(crate) crate::Object);

    impl Pointer {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Pointer {}

    impl wire::Interface for Pointer {
        const NAME: &'static str = "ei_pointer";
        const VERSION: u32 = 1;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Pointer {}

    impl Pointer {
        /// Pointer removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This object has been removed and a client should release all
        /// associated resources.
        ///
        /// This object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn destroyed(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(0, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }

        /// Relative motion event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_pointer.motion_relative request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than receiver.
        /// # Parameters
        ///
        /// - `x`
        /// - `y`
        ///
        pub fn motion_relative(&self, x: f32, y: f32) -> () {
            let args = &[wire::Arg::Float(x.into()), wire::Arg::Float(y.into())];

            self.0.request(1, args);

            ()
        }
    }

    /// All requests of interface `ei_pointer`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Pointer sub-interface removal request.
        ///
        /// Notification that the client is no longer interested in this pointer.
        /// The EIS implementation will release any resources related to this pointer and
        /// send the ei_pointer.destroyed event once complete.
        Release,
        /// Relative motion request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Generate a relative motion event on this pointer.
        ///
        /// It is a client bug to send this request more than once
        /// within the same ei_device.frame and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than sender.
        MotionRelative {
            /// The x movement in logical pixels.
            x: f32,
            /// The y movement in logical pixels.
            y: f32,
        },
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("release"),
                1 => Some("motion_relative"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => Ok(Self::Release),
                1 => {
                    let x = _bytes.read_arg()?;
                    let y = _bytes.read_arg()?;

                    Ok(Self::MotionRelative { x, y })
                }
                opcode => Err(wire::ParseError::InvalidOpcode("pointer", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::Release => {}
                Self::MotionRelative { x, y } => {
                    args.push(x.as_arg());
                    args.push(y.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use pointer::Pointer;

/// Device sub-interface for absolute pointer motion.
///
/// Server-side protocol definition module for interface `ei_pointer_absolute`.
///
/**
Interface for absolute pointer motion.

This interface is only provided once per device and where a client
requests ei_pointer_absolute.release the interface does not get
re-initialized. An EIS implementation may adjust the behavior of the
device (including removing the device) if the interface is released.
 */
pub mod pointer_absolute {
    use crate::wire;

    /// Device sub-interface for absolute pointer motion.
    ///
    /// Server-side interface proxy for interface `ei_pointer_absolute`.
    ///
    /**
    Interface for absolute pointer motion.

    This interface is only provided once per device and where a client
    requests ei_pointer_absolute.release the interface does not get
    re-initialized. An EIS implementation may adjust the behavior of the
    device (including removing the device) if the interface is released.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct PointerAbsolute(pub(crate) crate::Object);

    impl PointerAbsolute {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for PointerAbsolute {}

    impl wire::Interface for PointerAbsolute {
        const NAME: &'static str = "ei_pointer_absolute";
        const VERSION: u32 = 1;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for PointerAbsolute {}

    impl PointerAbsolute {
        /// Pointer absolute removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This object has been removed and a client should release all
        /// associated resources.
        ///
        /// This object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn destroyed(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(0, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }

        /// Absolute motion event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_pointer_absolute.motion_absolute request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than receiver.
        /// # Parameters
        ///
        /// - `x`
        /// - `y`
        ///
        pub fn motion_absolute(&self, x: f32, y: f32) -> () {
            let args = &[wire::Arg::Float(x.into()), wire::Arg::Float(y.into())];

            self.0.request(1, args);

            ()
        }
    }

    /// All requests of interface `ei_pointer_absolute`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Absolute pointer sub-interface removal request.
        ///
        /// Notification that the client is no longer interested in this object.
        /// The EIS implementation will release any resources related to this object and
        /// send the ei_pointer_absolute.destroyed event once complete.
        Release,
        /// Absolute motion request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Generate an absolute motion event on this pointer. The x/y
        /// coordinates must be within the device's regions or the event
        /// is silently discarded.
        ///
        /// It is a client bug to send this request more than once
        /// within the same ei_device.frame and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than sender.
        MotionAbsolute {
            /// The x position in logical pixels.
            x: f32,
            /// The y position in logical pixels.
            y: f32,
        },
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("release"),
                1 => Some("motion_absolute"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => Ok(Self::Release),
                1 => {
                    let x = _bytes.read_arg()?;
                    let y = _bytes.read_arg()?;

                    Ok(Self::MotionAbsolute { x, y })
                }
                opcode => Err(wire::ParseError::InvalidOpcode("pointer_absolute", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::Release => {}
                Self::MotionAbsolute { x, y } => {
                    args.push(x.as_arg());
                    args.push(y.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use pointer_absolute::PointerAbsolute;

/// Scroll object.
///
/// Server-side protocol definition module for interface `ei_scroll`.
///
/**
Interface for scroll requests and events.

This interface is only provided once per device and where a client
requests ei_scroll.release the interface does not get
re-initialized. An EIS implementation may adjust the behavior of the
device (including removing the device) if the interface is released.
 */
pub mod scroll {
    use crate::wire;

    /// Scroll object.
    ///
    /// Server-side interface proxy for interface `ei_scroll`.
    ///
    /**
    Interface for scroll requests and events.

    This interface is only provided once per device and where a client
    requests ei_scroll.release the interface does not get
    re-initialized. An EIS implementation may adjust the behavior of the
    device (including removing the device) if the interface is released.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Scroll(pub(crate) crate::Object);

    impl Scroll {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Scroll {}

    impl wire::Interface for Scroll {
        const NAME: &'static str = "ei_scroll";
        const VERSION: u32 = 1;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Scroll {}

    impl Scroll {
        /// Scroll removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This object has been removed and a client should release all
        /// associated resources.
        ///
        /// This object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn destroyed(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(0, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }

        /// Scroll event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_scroll.scroll request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than receiver.
        /// # Parameters
        ///
        /// - `x`
        /// - `y`
        ///
        pub fn scroll(&self, x: f32, y: f32) -> () {
            let args = &[wire::Arg::Float(x.into()), wire::Arg::Float(y.into())];

            self.0.request(1, args);

            ()
        }

        /// Discrete scroll event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_scroll.scroll_discrete request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than receiver.
        /// # Parameters
        ///
        /// - `x`
        /// - `y`
        ///
        pub fn scroll_discrete(&self, x: i32, y: i32) -> () {
            let args = &[wire::Arg::Int32(x.into()), wire::Arg::Int32(y.into())];

            self.0.request(2, args);

            ()
        }

        /// Scroll stop event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_scroll.scroll_stop request for details.
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than receiver.
        /// # Parameters
        ///
        /// - `x`
        /// - `y`
        /// - `is_cancel`
        ///
        pub fn scroll_stop(&self, x: u32, y: u32, is_cancel: u32) -> () {
            let args = &[
                wire::Arg::Uint32(x.into()),
                wire::Arg::Uint32(y.into()),
                wire::Arg::Uint32(is_cancel.into()),
            ];

            self.0.request(3, args);

            ()
        }
    }

    /// All requests of interface `ei_scroll`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Scroll removal request.
        ///
        /// Notification that the client is no longer interested in this object.
        /// The EIS implementation will release any resources related to this object and
        /// send the ei_scroll.destroyed event once complete.
        Release,
        /// Scroll request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Generate a a smooth (pixel-precise) scroll event on this pointer.
        /// Clients must not send ei_scroll.scroll_discrete events for the same event,
        /// the EIS implementation is responsible for emulation of discrete
        /// scroll events.
        ///
        /// It is a client bug to send this request more than once
        /// within the same ei_device.frame and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than sender.
        Scroll {
            /// The x movement in logical pixels.
            x: f32,
            /// The y movement in logical pixels.
            y: f32,
        },
        /// Scroll discrete request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Generate a a discrete (e.g. wheel) scroll event on this pointer.
        /// Clients must not send ei_scroll.scroll events for the same event,
        /// the EIS implementation is responsible for emulation of smooth
        /// scroll events.
        ///
        /// A discrete scroll event is based logical scroll units (equivalent to one
        /// mouse wheel click). The value for one scroll unit is 120, a fraction or
        /// multiple thereof represents a fraction or multiple of a wheel click.
        ///
        /// It is a client bug to send this request more than once
        /// within the same ei_device.frame and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than sender.
        ScrollDiscrete {
            /// The x movement in fractions or multiples of 120.
            x: i32,
            /// The y movement in fractions or multiples of 120.
            y: i32,
        },
        /// Scroll stop request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Generate a a scroll stop or cancel event on this pointer.
        ///
        /// A scroll stop event notifies the EIS implementation that the interaction causing a
        /// scroll motion previously triggered with ei_scroll.scroll or
        /// ei_scroll.scroll_discrete has stopped. For example, if all
        /// fingers are lifted off a touchpad, two-finger scrolling has logically
        /// stopped. The EIS implementation may use this information to e.g. start kinetic scrolling
        /// previously based on the previous finger speed.
        ///
        /// If is_cancel is nonzero, the event represents a cancellation of the
        /// current interaction. This indicates that the interaction has stopped to the
        /// point where further (server-emulated) scroll events from this device are wrong.
        ///
        /// It is a client bug to send this request more than once
        /// within the same ei_device.frame and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a client bug to send this request for an axis that
        /// had a a nonzero value in either ei_scroll.scroll or ei_scroll.scroll_discrete
        /// in the current frame and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than sender.
        ScrollStop {
            /// Nonzero if this axis stopped scrolling.
            x: u32,
            /// Nonzero if this axis stopped scrolling.
            y: u32,
            /// Nonzero to indicate this is a cancel event.
            is_cancel: u32,
        },
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("release"),
                1 => Some("scroll"),
                2 => Some("scroll_discrete"),
                3 => Some("scroll_stop"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => Ok(Self::Release),
                1 => {
                    let x = _bytes.read_arg()?;
                    let y = _bytes.read_arg()?;

                    Ok(Self::Scroll { x, y })
                }
                2 => {
                    let x = _bytes.read_arg()?;
                    let y = _bytes.read_arg()?;

                    Ok(Self::ScrollDiscrete { x, y })
                }
                3 => {
                    let x = _bytes.read_arg()?;
                    let y = _bytes.read_arg()?;
                    let is_cancel = _bytes.read_arg()?;

                    Ok(Self::ScrollStop { x, y, is_cancel })
                }
                opcode => Err(wire::ParseError::InvalidOpcode("scroll", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::Release => {}
                Self::Scroll { x, y } => {
                    args.push(x.as_arg());
                    args.push(y.as_arg());
                }
                Self::ScrollDiscrete { x, y } => {
                    args.push(x.as_arg());
                    args.push(y.as_arg());
                }
                Self::ScrollStop { x, y, is_cancel } => {
                    args.push(x.as_arg());
                    args.push(y.as_arg());
                    args.push(is_cancel.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use scroll::Scroll;

/// Button object.
///
/// Server-side protocol definition module for interface `ei_button`.
///
/**
Interface for button requests and events.

This interface is only provided once per device and where a client
requests ei_button.release the interface does not get
re-initialized. An EIS implementation may adjust the behavior of the
device (including removing the device) if the interface is released.
 */
pub mod button {
    use crate::wire;

    /// Button object.
    ///
    /// Server-side interface proxy for interface `ei_button`.
    ///
    /**
    Interface for button requests and events.

    This interface is only provided once per device and where a client
    requests ei_button.release the interface does not get
    re-initialized. An EIS implementation may adjust the behavior of the
    device (including removing the device) if the interface is released.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Button(pub(crate) crate::Object);

    impl Button {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Button {}

    impl wire::Interface for Button {
        const NAME: &'static str = "ei_button";
        const VERSION: u32 = 1;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Button {}

    impl Button {
        /// Pointer removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This pointer has been removed and a client should release all
        /// associated resources.
        ///
        /// This ei_scroll object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn destroyed(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(0, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }

        /// Button state change event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_scroll.button request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than receiver.
        ///
        /// It is an EIS implementation bug to send more than one button request
        /// for the same button within the same ei_device.frame.
        /// # Parameters
        ///
        /// - `button`
        /// - `state`
        ///
        pub fn button(&self, button: u32, state: ButtonState) -> () {
            let args = &[
                wire::Arg::Uint32(button.into()),
                wire::Arg::Uint32(state.into()),
            ];

            self.0.request(1, args);

            ()
        }
    }

    pub use crate::eiproto_enum::button::ButtonState;

    /// All requests of interface `ei_button`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Button removal request.
        ///
        /// Notification that the client is no longer interested in this object.
        /// The EIS implementation will release any resources related to this object and
        /// send the ei_button.destroyed event once complete.
        Release,
        /// Button state change request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Generate a button event on this pointer.
        ///
        /// The button codes must match the defines in linux/input-event-codes.h.
        ///
        /// It is a client bug to send more than one button request for the same button
        /// within the same ei_device.frame and the EIS implementation
        /// may ignore either or all button state changes and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than sender.
        Button {
            /// Button code.
            button: u32,
            /// .
            state: ButtonState,
        },
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("release"),
                1 => Some("button"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => Ok(Self::Release),
                1 => {
                    let button = _bytes.read_arg()?;
                    let state = _bytes.read_arg()?;

                    Ok(Self::Button { button, state })
                }
                opcode => Err(wire::ParseError::InvalidOpcode("button", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::Release => {}
                Self::Button { button, state } => {
                    args.push(button.as_arg());
                    args.push(state.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use button::Button;

/// Keyboard object.
///
/// Server-side protocol definition module for interface `ei_keyboard`.
///
/**
Interface for keyboard requests and events.

This interface is only provided once per device and where a client
requests ei_keyboard.release the interface does not get re-initialized. An
EIS implementation may adjust the behavior of the device (including removing
the device) if the interface is released.
 */
pub mod keyboard {
    use crate::wire;

    /// Keyboard object.
    ///
    /// Server-side interface proxy for interface `ei_keyboard`.
    ///
    /**
    Interface for keyboard requests and events.

    This interface is only provided once per device and where a client
    requests ei_keyboard.release the interface does not get re-initialized. An
    EIS implementation may adjust the behavior of the device (including removing
    the device) if the interface is released.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Keyboard(pub(crate) crate::Object);

    impl Keyboard {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Keyboard {}

    impl wire::Interface for Keyboard {
        const NAME: &'static str = "ei_keyboard";
        const VERSION: u32 = 1;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Keyboard {}

    impl Keyboard {
        /// Keyboard removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This keyboard has been removed and a client should release all
        /// associated resources.
        ///
        /// This ei_keyboard object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn destroyed(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(0, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }

        /// Keymap notification.
        ///
        /// Notification that this device has a keymap. Future key events must be
        /// interpreted by the client according to this keymap. For clients
        /// of ei_handshake.context_type sender it is the client's
        /// responsibility to send the correct ei_keyboard.key keycodes to
        /// generate the expected keysym in the EIS implementation.
        ///
        /// The keymap is constant for the lifetime of the device.
        ///
        /// This event provides a file descriptor to the client that can be
        /// memory-mapped in read-only mode to provide a keyboard mapping
        /// description. The fd must be mapped with MAP_PRIVATE by
        /// the recipient, as MAP_SHARED may fail.
        ///
        /// This event is optional and only sent immediately after the ei_keyboard object is created
        /// and before the ei_device.done event. It is a protocol violation to send this
        /// event after the ei_device.done event.
        /// # Parameters
        ///
        /// - `keymap_type`: The keymap type.
        /// - `size`: The keymap size in bytes.
        /// - `keymap`: File descriptor to the keymap.
        ///
        pub fn keymap(
            &self,
            keymap_type: KeymapType,
            size: u32,
            keymap: std::os::unix::io::BorrowedFd,
        ) -> () {
            let args = &[
                wire::Arg::Uint32(keymap_type.into()),
                wire::Arg::Uint32(size.into()),
                wire::Arg::Fd(keymap.into()),
            ];

            self.0.request(1, args);

            ()
        }

        /// Key state change event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_keyboard.key request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than receiver.
        ///
        /// It is a protocol violation to send a key down event in the same
        /// frame as a key up event for the same key in the same frame.
        /// # Parameters
        ///
        /// - `key`
        /// - `state`
        ///
        pub fn key(&self, key: u32, state: KeyState) -> () {
            let args = &[
                wire::Arg::Uint32(key.into()),
                wire::Arg::Uint32(state.into()),
            ];

            self.0.request(2, args);

            ()
        }

        /// Modifier change event.
        ///
        /// Notification that the EIS implementation has changed group or modifier
        /// states on this device, but not necessarily in response to an
        /// ei_keyboard.key event or request. Future ei_keyboard.key requests must
        /// take the new group and modifier state into account.
        ///
        /// This event should be sent any time the modifier state or effective group
        /// has changed, whether caused by an ei_keyboard.key event in accordance
        /// with the keymap, indirectly due to further handling of an
        /// ei_keyboard.key event (e.g., because it triggered a keyboard shortcut
        /// that then changed the state), or caused by an unrelated an event (e.g.,
        /// input from a different keyboard, or a group change triggered by a layout
        /// selection widget).
        ///
        /// For receiver clients, modifiers events will always be properly ordered
        /// with received key events, so each key event should be interpreted using
        /// the most recently-received modifier state. The EIS implementation should
        /// send this event immediately following the ei_device.frame event for the
        /// key press that caused the change. If the state change impacts multiple
        /// keyboards, this event should be sent for all of them.
        ///
        /// For sender clients, the modifiers event is not inherently synchronized
        /// with key requests, but the client may send an ei_connection.sync request
        /// when synchronization is required. When the corresponding
        /// ei_callback.done event is received, all key requests sent prior to the
        /// sync request are guaranteed to have been processed, and any
        /// directly-resulting modifiers events are guaranteed to have been
        /// received. Note, however, that it is still possible for
        /// indirectly-triggered state changes, such as via a keyboard shortcut not
        /// encoded in the keymap, to be reported after the done event.
        ///
        /// A client must assume that all modifiers are lifted when it
        /// receives an ei_device.paused event. The EIS implementation
        /// must send this event after ei_device.resumed to notify the client
        /// of any nonzero modifier state.
        ///
        /// This event does not require an ei_device.frame and should
        /// be processed immediately by the client.
        ///
        /// This event is only sent for devices with an ei_keyboard.keymap.
        ///
        /// Note: A previous version of the documentation instead specified that
        /// this event should not be sent in response to ei_keyboard.key events that
        /// change the group or modifier state according to the keymap. However,
        /// this complicated client implementation and resulted in situations where
        /// the client state could get out of sync with the EIS implementation.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        /// - `depressed`: Depressed modifiers.
        /// - `locked`: Locked modifiers.
        /// - `latched`: Latched modifiers.
        /// - `group`: The keyboard group (layout).
        ///
        pub fn modifiers(
            &self,
            serial: u32,
            depressed: u32,
            locked: u32,
            latched: u32,
            group: u32,
        ) -> () {
            let args = &[
                wire::Arg::Uint32(serial.into()),
                wire::Arg::Uint32(depressed.into()),
                wire::Arg::Uint32(locked.into()),
                wire::Arg::Uint32(latched.into()),
                wire::Arg::Uint32(group.into()),
            ];

            self.0.request(3, args);

            ()
        }
    }

    pub use crate::eiproto_enum::keyboard::KeyState;
    pub use crate::eiproto_enum::keyboard::KeymapType;

    /// All requests of interface `ei_keyboard`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Keyboard removal request.
        ///
        /// Notification that the client is no longer interested in this keyboard.
        /// The EIS implementation will release any resources related to this keyboard and
        /// send the ei_keyboard.destroyed event once complete.
        Release,
        /// Key state change request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Generate a key event on this keyboard. If the device has an
        /// ei_keyboard.keymap, the key code corresponds to that keymap.
        ///
        /// The key codes must match the defines in linux/input-event-codes.h.
        ///
        /// It is a client bug to send more than one key request for the same key
        /// within the same ei_device.frame and the EIS implementation
        /// may ignore either or all key state changes and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than sender.
        Key {
            /// The key code.
            key: u32,
            /// Logical state of the key.
            state: KeyState,
        },
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("release"),
                1 => Some("key"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => Ok(Self::Release),
                1 => {
                    let key = _bytes.read_arg()?;
                    let state = _bytes.read_arg()?;

                    Ok(Self::Key { key, state })
                }
                opcode => Err(wire::ParseError::InvalidOpcode("keyboard", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::Release => {}
                Self::Key { key, state } => {
                    args.push(key.as_arg());
                    args.push(state.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use keyboard::Keyboard;

/// Touchscreen object.
///
/// Server-side protocol definition module for interface `ei_touchscreen`.
///
/**
Interface for touchscreen requests and events.

This interface is only provided once per device and where a client
requests ei_touchscreen.release the interface does not get re-initialized. An
EIS implementation may adjust the behavior of the device (including removing
the device) if the interface is released.
 */
pub mod touchscreen {
    use crate::wire;

    /// Touchscreen object.
    ///
    /// Server-side interface proxy for interface `ei_touchscreen`.
    ///
    /**
    Interface for touchscreen requests and events.

    This interface is only provided once per device and where a client
    requests ei_touchscreen.release the interface does not get re-initialized. An
    EIS implementation may adjust the behavior of the device (including removing
    the device) if the interface is released.
     */
    #[derive(Clone, Debug, Hash, Eq, PartialEq)]
    pub struct Touchscreen(pub(crate) crate::Object);

    impl Touchscreen {
        /// Returns the negotiated version of the interface.
        pub fn version(&self) -> u32 {
            self.0.version()
        }

        /// Returns `true` if the backend has this object.
        pub fn is_alive(&self) -> bool {
            self.0.is_alive()
        }
    }

    impl crate::private::Sealed for Touchscreen {}

    impl wire::Interface for Touchscreen {
        const NAME: &'static str = "ei_touchscreen";
        const VERSION: u32 = 2;
        const CLIENT_SIDE: bool = false;

        fn new_unchecked(object: crate::Object) -> Self {
            Self(object)
        }

        fn as_object(&self) -> &crate::Object {
            &self.0
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            self.0.as_arg()
        }
    }

    impl crate::eis::Interface for Touchscreen {}

    impl Touchscreen {
        /// Touchscreen removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This touch has been removed and a client should release all
        /// associated resources.
        ///
        /// This ei_touchscreen object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        /// # Parameters
        ///
        /// - `serial`: This event's serial number.
        ///
        pub fn destroyed(&self, serial: u32) -> () {
            let args = &[wire::Arg::Uint32(serial.into())];

            self.0.request(0, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }

        /// Touch down event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_touchscreen.down request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than receiver.
        ///
        /// It is a protocol violation to send a touch down in the same
        /// frame as a touch motion or touch up.
        /// # Parameters
        ///
        /// - `touchid`
        /// - `x`
        /// - `y`
        ///
        pub fn down(&self, touchid: u32, x: f32, y: f32) -> () {
            let args = &[
                wire::Arg::Uint32(touchid.into()),
                wire::Arg::Float(x.into()),
                wire::Arg::Float(y.into()),
            ];

            self.0.request(1, args);

            ()
        }

        /// Touch motion event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_touchscreen.motion request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than receiver.
        ///
        /// It is a protocol violation to send a touch motion in the same
        /// frame as a touch down or touch up.
        /// # Parameters
        ///
        /// - `touchid`
        /// - `x`
        /// - `y`
        ///
        pub fn motion(&self, touchid: u32, x: f32, y: f32) -> () {
            let args = &[
                wire::Arg::Uint32(touchid.into()),
                wire::Arg::Float(x.into()),
                wire::Arg::Float(y.into()),
            ];

            self.0.request(2, args);

            ()
        }

        /// Touch motion event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_touchscreen.up request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an ei_handshake.context_type other than receiver.
        ///
        /// If a touch is released via ei_touchscreen.up, no ei_touchscreen.cancel
        /// event is sent for this same touch. Likewise, a touch released
        /// with ei_touchscreen.cancel must not be released via ei_touchscreen.up.
        ///
        /// It is a protocol violation to send a touch up in the same
        /// frame as a touch motion or touch down.
        /// # Parameters
        ///
        /// - `touchid`
        ///
        pub fn up(&self, touchid: u32) -> () {
            let args = &[wire::Arg::Uint32(touchid.into())];

            self.0.request(3, args);

            ()
        }

        /// Touch cancel event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::eis::handshake::ContextType).
        ///
        /// See the ei_touchscreen.cancel request for details.
        ///
        /// It is a protocol violation to send this event for a client
        /// of an ei_handshake.context_type other than receiver.
        ///
        /// If a touch is cancelled via ei_touchscreen.cancel, no ei_touchscreen.up
        /// event is sent for this same touch. Likewise, a touch released
        /// with ei_touchscreen.up must not be cancelled via ei_touchscreen.cancel.
        ///
        /// It is a protocol violation to send a touch cancel event in the same
        /// frame as a touch motion or touch down.
        /// # Parameters
        ///
        /// - `touchid`
        ///
        pub fn cancel(&self, touchid: u32) -> () {
            let args = &[wire::Arg::Uint32(touchid.into())];

            self.0.request(4, args);

            ()
        }
    }

    /// All requests of interface `ei_touchscreen`.
    ///
    /// Requests are messages that come from clients.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Request {
        /// Touch removal request.
        ///
        /// Notification that the client is no longer interested in this touchscreen.
        /// The EIS implementation will release any resources related to this touch and
        /// send the ei_touchscreen.destroyed event once complete.
        Release,
        /// Touch down request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Notifies the EIS implementation about a new touch logically down at the
        /// given coordinates. The touchid is a unique id for this touch. Touchids
        /// may be re-used after ei_touchscreen.up.
        ///
        /// The x/y coordinates must be within the device's regions or the event and future
        /// ei_touchscreen.motion events with the same touchid are silently discarded.
        ///
        /// It is a protocol violation to send a touch down in the same
        /// frame as a touch motion or touch up.
        Down {
            /// A unique touch id to identify this touch.
            touchid: u32,
            /// Touch x coordinate in logical pixels.
            x: f32,
            /// Touch y coordinate in logical pixels.
            y: f32,
        },
        /// Touch motion request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Notifies the EIS implementation about an existing touch changing position to
        /// the given coordinates. The touchid is the unique id for this touch previously
        /// sent with ei_touchscreen.down.
        ///
        /// The x/y coordinates must be within the device's regions or the event is
        /// silently discarded.
        ///
        /// It is a protocol violation to send a touch motion in the same
        /// frame as a touch down or touch up.
        Motion {
            /// A unique touch id to identify this touch.
            touchid: u32,
            /// Touch x coordinate in logical pixels.
            x: f32,
            /// Touch y coordinate in logical pixels.
            y: f32,
        },
        /// Touch up request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Notifies the EIS implementation about an existing touch being logically
        /// up. The touchid is the unique id for this touch previously
        /// sent with ei_touchscreen.down.
        ///
        /// If a touch is cancelled via ei_touchscreen.cancel, the ei_touchscreen.up
        /// request must not be sent for this same touch. Likewise, a touch released
        /// with ei_touchscreen.up must not be cancelled.
        ///
        /// The touchid may be re-used after this request.
        ///
        /// It is a protocol violation to send a touch up in the same
        /// frame as a touch motion or touch down.
        Up {
            /// A unique touch id to identify this touch.
            touchid: u32,
        },
        /// Touch cancel request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::eis::handshake::ContextType).
        ///
        /// Notifies the EIS implementation about an existing touch being cancelled.
        /// This typically means that any effects the touch may have had on the
        /// user interface should be reverted or otherwise made inconsequential.
        ///
        /// This request replaces ei_touchscreen.up for the same touch.
        /// If a touch is cancelled via ei_touchscreen.cancel, the ei_touchscreen.up
        /// request must not be sent for this same touch. Likewise, a touch released
        /// with ei_touchscreen.up must not be cancelled.
        ///
        /// The touchid is the unique id for this touch previously
        /// sent with ei_touchscreen.down.
        ///
        /// The touchid may be re-used after this request.
        ///
        /// It is a protocol violation to send a touch cancel
        /// in the same frame as a touch motion or touch down.
        Cancel {
            /// A unique touch id to identify this touch.
            touchid: u32,
        },
    }

    impl Request {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("release"),
                1 => Some("down"),
                2 => Some("motion"),
                3 => Some("up"),
                4 => Some("cancel"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => Ok(Self::Release),
                1 => {
                    let touchid = _bytes.read_arg()?;
                    let x = _bytes.read_arg()?;
                    let y = _bytes.read_arg()?;

                    Ok(Self::Down { touchid, x, y })
                }
                2 => {
                    let touchid = _bytes.read_arg()?;
                    let x = _bytes.read_arg()?;
                    let y = _bytes.read_arg()?;

                    Ok(Self::Motion { touchid, x, y })
                }
                3 => {
                    let touchid = _bytes.read_arg()?;

                    Ok(Self::Up { touchid })
                }
                4 => {
                    let touchid = _bytes.read_arg()?;

                    Ok(Self::Cancel { touchid })
                }
                opcode => Err(wire::ParseError::InvalidOpcode("touchscreen", opcode)),
            }
        }

        #[allow(
            unused_imports,
            unused_mut,
            unused_variables,
            unreachable_code,
            unreachable_patterns
        )]
        pub(super) fn args(&self) -> Vec<wire::Arg<'_>> {
            use crate::{wire::OwnedArg, Interface};
            let mut args = Vec::new();
            match self {
                Self::Release => {}
                Self::Down { touchid, x, y } => {
                    args.push(touchid.as_arg());
                    args.push(x.as_arg());
                    args.push(y.as_arg());
                }
                Self::Motion { touchid, x, y } => {
                    args.push(touchid.as_arg());
                    args.push(x.as_arg());
                    args.push(y.as_arg());
                }
                Self::Up { touchid } => {
                    args.push(touchid.as_arg());
                }
                Self::Cancel { touchid } => {
                    args.push(touchid.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use touchscreen::Touchscreen;

/// All requests of all interfaces.
///
/// Requests are messages that come from clients.
#[non_exhaustive]
#[derive(Debug)]
pub enum Request {
    Handshake(handshake::Handshake, handshake::Request),
    Connection(connection::Connection, connection::Request),
    Callback(callback::Callback, callback::Request),
    Pingpong(pingpong::Pingpong, pingpong::Request),
    Seat(seat::Seat, seat::Request),
    Device(device::Device, device::Request),
    Pointer(pointer::Pointer, pointer::Request),
    PointerAbsolute(pointer_absolute::PointerAbsolute, pointer_absolute::Request),
    Scroll(scroll::Scroll, scroll::Request),
    Button(button::Button, button::Request),
    Keyboard(keyboard::Keyboard, keyboard::Request),
    Touchscreen(touchscreen::Touchscreen, touchscreen::Request),
}

impl Request {
    pub(crate) fn op_name(interface: &str, operand: u32) -> Option<&'static str> {
        match interface {
            "ei_handshake" => handshake::Request::op_name(operand),
            "ei_connection" => connection::Request::op_name(operand),
            "ei_callback" => callback::Request::op_name(operand),
            "ei_pingpong" => pingpong::Request::op_name(operand),
            "ei_seat" => seat::Request::op_name(operand),
            "ei_device" => device::Request::op_name(operand),
            "ei_pointer" => pointer::Request::op_name(operand),
            "ei_pointer_absolute" => pointer_absolute::Request::op_name(operand),
            "ei_scroll" => scroll::Request::op_name(operand),
            "ei_button" => button::Request::op_name(operand),
            "ei_keyboard" => keyboard::Request::op_name(operand),
            "ei_touchscreen" => touchscreen::Request::op_name(operand),
            _ => None,
        }
    }

    pub(crate) fn parse(
        object: crate::Object,
        operand: u32,
        bytes: &mut wire::ByteStream,
    ) -> Result<Self, wire::ParseError> {
        match object.interface() {
            "ei_handshake" => Ok(Self::Handshake(
                object.downcast_unchecked(),
                handshake::Request::parse(operand, bytes)?,
            )),
            "ei_connection" => Ok(Self::Connection(
                object.downcast_unchecked(),
                connection::Request::parse(operand, bytes)?,
            )),
            "ei_callback" => Ok(Self::Callback(
                object.downcast_unchecked(),
                callback::Request::parse(operand, bytes)?,
            )),
            "ei_pingpong" => Ok(Self::Pingpong(
                object.downcast_unchecked(),
                pingpong::Request::parse(operand, bytes)?,
            )),
            "ei_seat" => Ok(Self::Seat(
                object.downcast_unchecked(),
                seat::Request::parse(operand, bytes)?,
            )),
            "ei_device" => Ok(Self::Device(
                object.downcast_unchecked(),
                device::Request::parse(operand, bytes)?,
            )),
            "ei_pointer" => Ok(Self::Pointer(
                object.downcast_unchecked(),
                pointer::Request::parse(operand, bytes)?,
            )),
            "ei_pointer_absolute" => Ok(Self::PointerAbsolute(
                object.downcast_unchecked(),
                pointer_absolute::Request::parse(operand, bytes)?,
            )),
            "ei_scroll" => Ok(Self::Scroll(
                object.downcast_unchecked(),
                scroll::Request::parse(operand, bytes)?,
            )),
            "ei_button" => Ok(Self::Button(
                object.downcast_unchecked(),
                button::Request::parse(operand, bytes)?,
            )),
            "ei_keyboard" => Ok(Self::Keyboard(
                object.downcast_unchecked(),
                keyboard::Request::parse(operand, bytes)?,
            )),
            "ei_touchscreen" => Ok(Self::Touchscreen(
                object.downcast_unchecked(),
                touchscreen::Request::parse(operand, bytes)?,
            )),
            intr => Err(wire::ParseError::InvalidInterface(intr.to_owned())),
        }
    }
}

impl wire::MessageEnum for Request {
    fn args(&self) -> Vec<wire::Arg<'_>> {
        match self {
            Self::Handshake(_, x) => x.args(),
            Self::Connection(_, x) => x.args(),
            Self::Callback(_, x) => x.args(),
            Self::Pingpong(_, x) => x.args(),
            Self::Seat(_, x) => x.args(),
            Self::Device(_, x) => x.args(),
            Self::Pointer(_, x) => x.args(),
            Self::PointerAbsolute(_, x) => x.args(),
            Self::Scroll(_, x) => x.args(),
            Self::Button(_, x) => x.args(),
            Self::Keyboard(_, x) => x.args(),
            Self::Touchscreen(_, x) => x.args(),
        }
    }
}
