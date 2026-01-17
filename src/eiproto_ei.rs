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
/// Client-side protocol definition module for interface `ei_handshake`.
///
/**
This is a special interface to setup the client as seen by the EIS
implementation. The object for this interface has the fixed object
id 0 and only exists until the connection has been set up, see the
`ei_handshake.connection` event.

The `ei_handshake` version is 1 until:
- the EIS implementation sends the handshake_version event with
  a version other than 1, and, in response,
- the client sends the handshake_version request with a
  version equal or lower to the EIS implementation version.

The EIS implementation must send the handshake_version event immediately
once the physical connection has been established.

Once the `ei_connection.connection` event has been sent the handshake
is destroyed by the EIS implementation.
 */
pub mod handshake {
    use crate::wire;

    /// Handshake object.
    ///
    /// Client-side interface proxy for interface `ei_handshake`.
    ///
    /**
    This is a special interface to setup the client as seen by the EIS
    implementation. The object for this interface has the fixed object
    id 0 and only exists until the connection has been set up, see the
    `ei_handshake.connection` event.

    The `ei_handshake` version is 1 until:
    - the EIS implementation sends the handshake_version event with
      a version other than 1, and, in response,
    - the client sends the handshake_version request with a
      version equal or lower to the EIS implementation version.

    The EIS implementation must send the handshake_version event immediately
    once the physical connection has been established.

    Once the `ei_connection.connection` event has been sent the handshake
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Handshake {}

    impl Handshake {
        /// Handshake version information from ei client.
        ///
        /// Informs the EIS implementation that this client supports the given
        /// version of the `ei_handshake` interface. The version number must be less
        /// than or equal to the version in the handshake_version event sent by the
        /// EIS implementation when the connection was established.
        ///
        /// Immediately after sending this request, the client must assume the negotiated
        /// version number for the `ei_handshake` interface and the EIS implementation
        /// may send events and process requests matching that version.
        ///
        /// This request must be sent exactly once and it must be the first request
        /// the client sends.
        /// # Parameters
        ///
        /// - `version`: The interface version.
        ///
        pub fn handshake_version(&self, version: u32) -> () {
            let args = &[wire::Arg::Uint32(version.into())];

            self.0.request(0, args);

            ()
        }

        /// Setup completion request.
        ///
        /// Informs the EIS implementation that configuration is complete.
        ///
        /// In the future (and possibly after requiring user interaction),
        /// the EIS implementation responds by sending the `ei_handshake.connection` event.
        pub fn finish(&self) -> () {
            let args = &[];

            self.0.request(1, args);

            ()
        }

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
        /// `ei_handshake.finish`.
        /// # Parameters
        ///
        /// - `context_type`: The connection's context type.
        ///
        pub fn context_type(&self, context_type: ContextType) -> () {
            let args = &[wire::Arg::Uint32(context_type.into())];

            self.0.request(2, args);

            ()
        }

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
        /// `ei_handshake.finish`.
        /// # Parameters
        ///
        /// - `name`: The client name.
        ///
        pub fn name(&self, name: &str) -> () {
            let args = &[wire::Arg::String(name.into())];

            self.0.request(3, args);

            ()
        }

        /// Interface support information.
        ///
        /// Informs the EIS implementation that the EI client supports the given
        /// named interface with the given maximum version number.
        ///
        /// Future objects created by the EIS implementation will
        /// use the respective interface version (or any lesser version)
        /// as announced by the `ei_connection.interface_version` event.
        ///
        /// This request must be sent for the "`ei_connection`" interface,
        /// failing to do so will result in the EIS implementation disconnecting
        /// the client on `ei_handshake.finish`.
        ///
        /// This request must not be sent for the "`ei_handshake`" interface, use
        /// the `ei_handshake.handshake_version` request instead.
        ///
        /// Note that an EIS implementation may consider some interfaces to
        /// be required and immediately `ei_connection.disconnect` a client
        /// not supporting those interfaces.
        ///
        /// This request must not be sent more than once per interface and must be
        /// sent before `ei_handshake.finish`.
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

            self.0.request(4, args);

            ()
        }
    }

    pub use crate::eiproto_enum::handshake::ContextType;

    /// All events of interface `ei_handshake`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /// Handshake version information from eis implementation.
        ///
        /// Informs the client that the EIS implementation supports the given
        /// version of the `ei_handshake` interface.
        ///
        /// This event is sent exactly once and immediately after connection to the
        /// EIS implementation.
        ///
        /// In response, the client must send the `ei_handshake.handshake_version` request
        /// with any version up to including the version provided in this event.
        /// See the `ei_handshake.handshake_version` request for details on what happens next.
        HandshakeVersion {
            /// The interface version.
            version: u32,
        },
        /// Interface support event.
        ///
        /// Informs the client that the EIS implementation supports the given named
        /// interface with the given maximum version number.
        ///
        /// The client must not assume those interfaces are supported unless
        /// and until those versions have been received.
        ///
        /// This request must not be sent for the "`ei_handshake`" interface, use
        /// the handshake_version event instead.
        ///
        /// This event may be sent by the EIS implementation for any
        /// other supported interface (but not necessarily all supported
        /// interfaces) before the `ei_handshake.connection` event.
        InterfaceVersion {
            /// The interface name.
            name: String,
            /// The interface version.
            version: u32,
        },
        /// Provides the core connection object.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// Provides the client with the connection object that is the top-level
        /// object for all future requests and events.
        ///
        /// This event must be sent exactly once after the client sends the
        /// `ei_handshake.finish` request to the EIS implementation.
        ///
        /// The `ei_handshake` object will be destroyed by the EIS implementation
        /// immediately after this event has been sent, the client must not attempt
        /// to use it after that point.
        ///
        /// The version sent by the EIS implementation is the version of the ``ei_connection``
        /// interface as announced by `ei_handshake.interface_version`, or any
        /// lower version.
        ///
        /// The serial number is the start value of the EIS implementation's serial
        /// number sequence. Clients must not assume any specific value for this
        /// serial number. Any future serial number in any event is monotonically
        /// increasing by an unspecified amount.
        Connection {
            /// This event's serial number.
            serial: u32,
            /// The connection object.
            connection: super::connection::Connection,
        },
    }

    impl Event {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("handshake_version"),
                1 => Some("interface_version"),
                2 => Some("connection"),
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
                1 => {
                    let name = _bytes.read_arg()?;
                    let version = _bytes.read_arg()?;

                    Ok(Self::InterfaceVersion { name, version })
                }
                2 => {
                    let serial = _bytes.read_arg()?;
                    let connection = _bytes.read_arg()?;
                    let version = _bytes.read_arg()?;

                    Ok(Self::Connection {
                        serial,
                        connection: _bytes.backend().new_peer_interface(connection, version)?,
                    })
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
                Self::InterfaceVersion { name, version } => {
                    args.push(name.as_arg());
                    args.push(version.as_arg());
                }
                Self::Connection { serial, connection } => {
                    args.push(serial.as_arg());
                    args.push(connection.as_arg());
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
/// Client-side protocol definition module for interface `ei_connection`.
///
/**
The core connection object. This is the top-level object for any communication
with the EIS implementation.

Note that for a client to receive this object, it must announce
support for this interface in `ei_handshake.interface_version`.
 */
pub mod connection {
    use crate::wire;

    /// Core connection object.
    ///
    /// Client-side interface proxy for interface `ei_connection`.
    ///
    /**
    The core connection object. This is the top-level object for any communication
    with the EIS implementation.

    Note that for a client to receive this object, it must announce
    support for this interface in `ei_handshake.interface_version`.
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Connection {}

    impl Connection {
        /// Asynchronous roundtrip.
        ///
        /// Requests the EIS implementation to emit the `ei_callback.done` event on
        /// the returned `ei_callback` object. Since requests are handled in-order
        /// and events are delivered in-order, this can be used as a
        /// synchronization point to ensure all previous requests and the resulting
        /// events have been handled.
        ///
        /// The object returned by this request will be destroyed by the
        /// EIS implementation after the callback is fired and as such the client must not
        /// attempt to use it after that point.
        ///
        /// The callback_data in the `ei_callback.done` event must be zero.
        ///
        /// Note that for a client to use this request it must announce
        /// support for the ``ei_callback`` interface in `ei_handshake.interface_version`.
        /// It is a protocol violation to request sync without having announced the
        /// ``ei_callback`` interface and the EIS implementation must disconnect
        /// the client.
        /// # Parameters
        ///
        /// - `version`: The interface version.
        ///
        pub fn sync(&self, version: u32) -> (super::callback::Callback) {
            let callback = self
                .0
                .backend_weak()
                .new_object("ei_callback".to_string(), version);
            let args = &[
                wire::Arg::NewId(callback.id().into()),
                wire::Arg::Uint32(version.into()),
            ];

            self.0.request(0, args);

            (super::callback::Callback(callback))
        }

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
        /// `ei_connection` object and it should close the socket. The EIS implementation
        /// will treat the connection as already disconnected on receipt and does not
        /// send the `ei_connection.disconnect` event in response to this request.
        pub fn disconnect(&self) -> () {
            let args = &[];

            self.0.request(1, args);
            self.0.backend_weak().remove_id(self.0.id());

            ()
        }
    }

    pub use crate::eiproto_enum::connection::DisconnectReason;

    /// All events of interface `ei_connection`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
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
        /// The `ei_connection` object will be destroyed by the
        /// EIS implementation immediately after this event has been sent, a
        /// client must not attempt to use it after that point.
        ///
        /// There is no guarantee this event is sent - the connection may be closed
        /// without a disconnection event.
        Disconnected {
            /// The last serial sent by the eis implementation.
            last_serial: u32,
            /// The reason for being disconnected.
            reason: DisconnectReason,
            /// An explanation for debugging purposes.
            explanation: Option<String>,
        },
        /// Seat presence information.
        ///
        /// Informs the client that a new seat has been added.
        ///
        /// A seat is a set of input devices that logically belong together.
        ///
        /// This event is only sent if the client announced support for the
        /// "`ei_seat`" interface in `ei_handshake.interface_version`.
        /// The interface version is equal or less to the client-supported
        /// version in `ei_handshake.interface_version` for the "`ei_seat`"
        /// interface.
        Seat {
            /// .
            seat: super::seat::Seat,
        },
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
        /// `ei_device.destroyed` and destroy the device's resources (and protocol object)
        /// at the same time as the client attempts to `ei_device.start_emulating`
        /// on that object.
        ///
        /// It is the client's responsibility to unwind any state changes done
        /// to the object since the last successful message.
        InvalidObject {
            /// The last serial sent by the eis implementation.
            last_serial: u32,
            /// .
            invalid_id: u64,
        },
        /// Ping event.
        ///
        /// The ping event asks the client to emit the 'done' event
        /// on the provided `ei_pingpong` object. Since requests are
        /// handled in-order and events are delivered in-order, this can
        /// be used as a synchronization point to ensure all previous requests
        /// and the resulting events have been handled.
        ///
        /// The object returned by this request must be destroyed by the
        /// ei client implementation after the callback is fired and as
        /// such the client must not attempt to use it after that point.
        ///
        /// The callback_data in the resulting `ei_pingpong.done` request is
        /// ignored by the EIS implementation.
        ///
        /// Note that for a EIS implementation to use this request the client must
        /// announce support for this interface in `ei_handshake.interface_version`. It is
        /// a protocol violation to send this event to a client without the
        /// "`ei_pingpong`" interface.
        Ping {
            /// Callback object for the ping request.
            ping: super::pingpong::Pingpong,
        },
    }

    impl Event {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("disconnected"),
                1 => Some("seat"),
                2 => Some("invalid_object"),
                3 => Some("ping"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => {
                    let last_serial = _bytes.read_arg()?;
                    let reason = _bytes.read_arg()?;
                    let explanation = _bytes.read_arg()?;

                    Ok(Self::Disconnected {
                        last_serial,
                        reason,
                        explanation,
                    })
                }
                1 => {
                    let seat = _bytes.read_arg()?;
                    let version = _bytes.read_arg()?;

                    Ok(Self::Seat {
                        seat: _bytes.backend().new_peer_interface(seat, version)?,
                    })
                }
                2 => {
                    let last_serial = _bytes.read_arg()?;
                    let invalid_id = _bytes.read_arg()?;

                    Ok(Self::InvalidObject {
                        last_serial,
                        invalid_id,
                    })
                }
                3 => {
                    let ping = _bytes.read_arg()?;
                    let version = _bytes.read_arg()?;

                    Ok(Self::Ping {
                        ping: _bytes.backend().new_peer_interface(ping, version)?,
                    })
                }
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
                Self::Disconnected {
                    last_serial,
                    reason,
                    explanation,
                } => {
                    args.push(last_serial.as_arg());
                    args.push(reason.as_arg());
                    args.push(explanation.as_arg());
                }
                Self::Seat { seat } => {
                    args.push(seat.as_arg());
                }
                Self::InvalidObject {
                    last_serial,
                    invalid_id,
                } => {
                    args.push(last_serial.as_arg());
                    args.push(invalid_id.as_arg());
                }
                Self::Ping { ping } => {
                    args.push(ping.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use connection::Connection;

/// Callback object.
///
/// Client-side protocol definition module for interface `ei_callback`.
///
/**
Interface for ensuring a roundtrip to the EIS implementation.
Clients can handle the 'done' event to get notified when
the related request that created the `ei_callback` object is done.
 */
pub mod callback {
    use crate::wire;

    /// Callback object.
    ///
    /// Client-side interface proxy for interface `ei_callback`.
    ///
    /**
    Interface for ensuring a roundtrip to the EIS implementation.
    Clients can handle the 'done' event to get notified when
    the related request that created the `ei_callback` object is done.
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Callback {}

    impl Callback {}

    /// All events of interface `ei_callback`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /// Done event.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// Informs the client that the associated request is finished. The EIS
        /// implementation must destroy the `ei_callback` object immediately after
        /// sending this event this event and as such the client must not attempt to
        /// use it after that point.
        Done {
            /// Request-specific data for the callback.
            callback_data: u64,
        },
    }

    impl Event {
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
                Self::Done { callback_data } => {
                    args.push(callback_data.as_arg());
                }
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use callback::Callback;

/// Callback object.
///
/// Client-side protocol definition module for interface `ei_pingpong`.
///
/**
Interface for ensuring a roundtrip to the client implementation.
This interface is identical to `ei_callback` but is intended for
the EIS implementation to enforce a roundtrip to the client.
 */
pub mod pingpong {
    use crate::wire;

    /// Callback object.
    ///
    /// Client-side interface proxy for interface `ei_pingpong`.
    ///
    /**
    Interface for ensuring a roundtrip to the client implementation.
    This interface is identical to `ei_callback` but is intended for
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Pingpong {}

    impl Pingpong {
        /// Done event.
        ///
        /// **Note:** This request is a destructor.
        ///
        /// Informs the EIS implementation when the associated event is finished.
        /// The client must destroy the `ei_pingpong` object immediately after this
        /// request and as such the server must not attempt to use it after that
        /// point.
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

    /// All events of interface `ei_pingpong`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {}

    impl Event {
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
                _ => unreachable!(),
            }
            args
        }
    }
}

pub use pingpong::Pingpong;

/// Set of input devices that logically belong together.
///
/// Client-side protocol definition module for interface `ei_seat`.
///
/**
An `ei_seat` represents a set of input devices that logically belong together. In most
cases only one seat is present and all input devices on that seat share the same
pointer and keyboard focus.

A seat has potential capabilities, a client is expected to bind to those capabilities.
The EIS implementation then creates logical input devices based on the capabilities the
client is interested in.

Immediately after creation of the `ei_seat` object, the EIS implementation sends a burst
of events with information about this seat. This burst of events is terminated by the
`ei_seat.done` event.
 */
pub mod seat {
    use crate::wire;

    /// Set of input devices that logically belong together.
    ///
    /// Client-side interface proxy for interface `ei_seat`.
    ///
    /**
    An `ei_seat` represents a set of input devices that logically belong together. In most
    cases only one seat is present and all input devices on that seat share the same
    pointer and keyboard focus.

    A seat has potential capabilities, a client is expected to bind to those capabilities.
    The EIS implementation then creates logical input devices based on the capabilities the
    client is interested in.

    Immediately after creation of the `ei_seat` object, the EIS implementation sends a burst
    of events with information about this seat. This burst of events is terminated by the
    `ei_seat.done` event.
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Seat {}

    impl Seat {
        /// Seat removal request.
        ///
        /// Informs the EIS implementation that the client is no longer interested
        /// in this seat. The EIS implementation should release any resources
        /// associated with this seat and send the `ei_seat.destroyed` event once
        /// finished.
        ///
        /// Note that releasing a seat does not guarantee that another seat becomes
        /// available. In other words, in most single-seat cases, releasing the seat
        /// means that the connection becomes effectively inert.
        pub fn release(&self) -> () {
            let args = &[];

            self.0.request(0, args);

            ()
        }

        /// Seat binding.
        ///
        /// Binds to the given bitmask of capabilities. Each one of the bit values
        /// in the given bitmask must originate from one of the `ei_seat.capability`
        /// events. See its documentation for more examples.
        ///
        /// The EIS implementation should return compatible devices with
        /// `ei_seat.device` events.
        ///
        /// Binding masks that are not supported in the `ei_device`'s interface
        /// version is a client bug and may result in disconnection.
        ///
        /// A client may send this request multiple times to adjust the capabilities
        /// it is interested in. If previously-bound capabilities are dropped by the
        /// client, the EIS implementation may `ei_device.remove` devices that have
        /// these capabilities.
        /// # Parameters
        ///
        /// - `capabilities`: Bitmask of the capabilities.
        ///
        pub fn bind(&self, capabilities: u64) -> () {
            let args = &[wire::Arg::Uint64(capabilities.into())];

            self.0.request(1, args);

            ()
        }
    }

    /// All events of interface `ei_seat`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /// Seat removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// Informs the client that this seat has been removed, and that it should
        /// release all associated resources.
        ///
        /// This `ei_seat` object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        Destroyed {
            /// This event's serial number.
            serial: u32,
        },
        /// Seat name notification.
        ///
        /// The name of this seat, if any. This event is optional and sent once immediately
        /// after object creation.
        ///
        /// It is a protocol violation to send this event after the `ei_seat.done` event.
        Name {
            /// The seat name.
            name: String,
        },
        /// Seat capability notification.
        ///
        /// Informs the client that this seat supports devices with the given
        /// interface. The interface must be mapped to a bitmask by the EIS
        /// implementation. The client may apply the binary OR operation onto these
        /// bitmasks in `ei_seat.bind`. In response, the EIS implementation may then
        /// create devices based on those bound capabilities.
        ///
        /// For example, an EIS implementation may advertise support for
        /// ``ei_pointer`` devices at bitmask `0x1`, ``ei_keyboard`` devices at `0x4`
        /// and ``ei_touchscreen`` devices at `0x8`. A client may then execute the
        /// request ``ei_seat.bind`(0xC)` to bind to keyboard and touchscreen devices
        /// but not pointing devices.
        ///
        /// The EIS implementation must not advertise capabilities for interfaces
        /// that have not been negotiated in the `ei_handshake` object.
        ///
        /// The EIS implementation may decide which capabilities a given seat has.
        /// After `ei_seat.done`, the capabilities are constant for the lifetime of
        /// the seat but may differ between seats. The masks may be sparse bitwise.
        ///
        /// This event is sent multiple time for each supported interface, finishing
        /// with `ei_seat.done`.
        Capability {
            /// The mask representing this capability.
            mask: u64,
            /// The interface name for this capability.
            interface: String,
        },
        /// Seat setup completion notification.
        ///
        /// Notification that the initial burst of events is complete and
        /// the client can set up this seat now.
        ///
        /// It is a protocol violation to send this event more than once.
        Done,
        /// Device presence notification.
        ///
        /// Informs the client that a new device has been added to the seat.
        ///
        /// The EIS implementation must never announce devices that have not been bound to with `ei_seat.bind`.
        ///
        /// This event is only sent if the client announced support for the
        /// ``ei_device`` interface in `ei_handshake.interface_version`. The interface
        /// version is less than or equal to the client-supported version in
        /// `ei_handshake.interface_version` for the ``ei_device`` interface.
        Device {
            /// The new device.
            device: super::device::Device,
        },
    }

    impl Event {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("destroyed"),
                1 => Some("name"),
                2 => Some("capability"),
                3 => Some("done"),
                4 => Some("device"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::Destroyed { serial })
                }
                1 => {
                    let name = _bytes.read_arg()?;

                    Ok(Self::Name { name })
                }
                2 => {
                    let mask = _bytes.read_arg()?;
                    let interface = _bytes.read_arg()?;

                    Ok(Self::Capability { mask, interface })
                }
                3 => Ok(Self::Done),
                4 => {
                    let device = _bytes.read_arg()?;
                    let version = _bytes.read_arg()?;

                    Ok(Self::Device {
                        device: _bytes.backend().new_peer_interface(device, version)?,
                    })
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
                Self::Destroyed { serial } => {
                    args.push(serial.as_arg());
                }
                Self::Name { name } => {
                    args.push(name.as_arg());
                }
                Self::Capability { mask, interface } => {
                    args.push(mask.as_arg());
                    args.push(interface.as_arg());
                }
                Self::Done => {}
                Self::Device { device } => {
                    args.push(device.as_arg());
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
/// Client-side protocol definition module for interface `ei_device`.
///
/**
An `ei_device` represents a single logical input device. Like physical input
devices an `ei_device` may have multiple capabilities and may e.g. function
as pointer and keyboard.

Depending on the `ei_handshake.context_type`, an `ei_device` can
emulate events via client requests or receive events. It is a protocol violation
to emulate certain events on a receiver device, or for the EIS implementation
to send certain events to the device. See the individual request/event documentation
for details.
 */
pub mod device {
    use crate::wire;

    /// Logical input device.
    ///
    /// Client-side interface proxy for interface `ei_device`.
    ///
    /**
    An `ei_device` represents a single logical input device. Like physical input
    devices an `ei_device` may have multiple capabilities and may e.g. function
    as pointer and keyboard.

    Depending on the `ei_handshake.context_type`, an `ei_device` can
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Device {}

    impl Device {
        /// Device removal request.
        ///
        /// Notification that the client is no longer interested in this device.
        ///
        /// Note that releasing a device does not guarantee another device becomes available.
        ///
        /// The EIS implementation will release any resources related to this device and
        /// send the `ei_device.destroyed` event once complete.
        pub fn release(&self) -> () {
            let args = &[];

            self.0.request(0, args);

            ()
        }

        /// Device start emulating request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Notify the EIS implementation that the given device is about to start
        /// sending events. This should be seen more as a transactional boundary than a
        /// time-based boundary. The primary use-cases for this are to allow for setup on
        /// the EIS implementation side and/or UI updates to indicate that a device is
        /// sending events now and for out-of-band information to sync with a given event
        /// sequence.
        ///
        /// There is no actual requirement that events start immediately once emulation
        /// starts and there is no requirement that a client calls `ei_device.stop_emulating`
        /// after the most recent events.
        /// For example, in a remote desktop use-case the client would call
        /// `ei_device.start_emulating` once the remote desktop session starts (rather than when
        /// the device sends events) and `ei_device.stop_emulating` once the remote desktop
        /// session stops.
        ///
        /// The sequence number identifies this transaction between start/stop emulating.
        /// It must go up by at least 1 on each call to `ei_device.start_emulating`.
        /// Wraparound must be handled by the EIS implementation but callers must ensure
        /// that detection of wraparound is possible.
        ///
        /// It is a protocol violation to request `ei_device.start_emulating` after
        /// `ei_device.start_emulating` without an intermediate stop_emulating.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than sender.
        /// # Parameters
        ///
        /// - `last_serial`: The last serial sent by the eis implementation.
        /// - `sequence`: Sequence number to identify this emulation sequence.
        ///
        pub fn start_emulating(&self, last_serial: u32, sequence: u32) -> () {
            let args = &[
                wire::Arg::Uint32(last_serial.into()),
                wire::Arg::Uint32(sequence.into()),
            ];

            self.0.request(1, args);

            ()
        }

        /// Device start emulating request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Notify the EIS implementation that the given device is no longer sending
        /// events. See `ei_device.start_emulating` for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than sender.
        /// # Parameters
        ///
        /// - `last_serial`: The last serial sent by the eis implementation.
        ///
        pub fn stop_emulating(&self, last_serial: u32) -> () {
            let args = &[wire::Arg::Uint32(last_serial.into())];

            self.0.request(2, args);

            ()
        }

        /// Device frame request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Generate a frame event to group the current set of events
        /// into a logical hardware event. This function must be called after one
        /// or more events on any of `ei_pointer`, `ei_pointer_absolute`,
        /// `ei_scroll`, `ei_button`, `ei_keyboard` or `ei_touchscreen` has
        /// been requested by the EIS implementation.
        ///
        /// The EIS implementation should not process changes to the device state
        /// until the `ei_device.frame` event. For example, pressing and releasing
        /// a key within the same frame is a logical noop.
        ///
        /// The given timestamp applies to all events in the current frame.
        /// The timestamp must be in microseconds of CLOCK_MONOTONIC.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than sender.
        /// # Parameters
        ///
        /// - `last_serial`: The last serial sent by the eis implementation.
        /// - `timestamp`: Timestamp in microseconds.
        ///
        pub fn frame(&self, last_serial: u32, timestamp: u64) -> () {
            let args = &[
                wire::Arg::Uint32(last_serial.into()),
                wire::Arg::Uint64(timestamp.into()),
            ];

            self.0.request(3, args);

            ()
        }
    }

    pub use crate::eiproto_enum::device::DeviceType;

    /// All events of interface `ei_device`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /// Device removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This device has been removed and a client should release all
        /// associated resources.
        ///
        /// This `ei_device` object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        Destroyed {
            /// This event's serial number.
            serial: u32,
        },
        /// Device name notification.
        ///
        /// The name of this device, if any. This event is optional and sent once immediately
        /// after object creation.
        ///
        /// It is a protocol violation to send this event after the `ei_device.done` event.
        Name {
            /// The device name.
            name: String,
        },
        /// Device type notification.
        ///
        /// The device type, one of virtual or physical.
        ///
        /// Devices of type `ei_device.device_type.physical` are only supported for
        /// clients of type `ei_handshake.context_type.receiver`.
        ///
        /// This event is sent once immediately after object creation.
        /// It is a protocol violation to send this event after the `ei_device.done` event.
        DeviceType {
            /// The device type.
            device_type: DeviceType,
        },
        /// Device dimensions notification.
        ///
        /// The device dimensions in mm. This event is optional and sent once immediately
        /// after object creation.
        ///
        /// This event is only sent for devices of `ei_device.device_type.physical`.
        ///
        /// It is a protocol violation to send this event after the `ei_device.done` event.
        Dimensions {
            /// The device physical width in mm.
            width: u32,
            /// The device physical height in mm.
            height: u32,
        },
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
        /// on a device_type.virtual device without advertising an `ei_region` for this device.
        ///
        /// This event is optional and sent immediately after object creation. Where a device
        /// has multiple regions, this event is sent once for each region.
        /// It is a protocol violation to send this event after the `ei_device.done` event.
        ///
        /// Note: the fourth argument ('hight') was misspelled when the protocol was declared
        /// stable but changing the name is an API breaking change.
        Region {
            /// Region x offset in logical pixels.
            offset_x: u32,
            /// Region y offset in logical pixels.
            offset_y: u32,
            /// Region width in logical pixels.
            width: u32,
            /// Region height in logical pixels.
            hight: u32,
            /// The physical scale for this region.
            scale: f32,
        },
        /// Device capability notification.
        ///
        /// Notification that a new device has a sub-interface.
        ///
        /// This event may be sent for the following interfaces:
        /// - "`ei_pointer`"
        /// - "`ei_pointer_absolute`"
        /// - "`ei_scroll`"
        /// - "`ei_button`"
        /// - "`ei_keyboard`"
        /// - "`ei_touchscreen`"
        /// The interface version is equal or less to the client-supported
        /// version in `ei_handshake.interface_version` for the respective interface.
        ///
        /// It is a protocol violation to send a notification for an interface that
        /// the client has not bound to with `ei_seat.bind`.
        ///
        /// This event is optional and sent immediately after object creation
        /// and at most once per interface.
        /// It is a protocol violation to send this event after the `ei_device.done` event.
        Interface {
            /// .
            object: crate::Object,
        },
        /// Device setup completion notification.
        ///
        /// Notification that the initial burst of events is complete and
        /// the client can set up this device now.
        ///
        /// It is a protocol violation to send this event more than once per device.
        Done,
        /// Device resumed notification.
        ///
        /// Notification that the device has been resumed by the EIS implementation
        /// and (depending on the `ei_handshake.context_type`) the client may request
        /// `ei_device.start_emulating` or the EIS implementation may
        /// `ei_device.start_emulating` events.
        ///
        /// It is a client bug to request emulation of events on a device that is
        /// not resumed. The EIS implementation may silently discard such events.
        ///
        /// A newly advertised device is in the `ei_device.paused` state.
        Resumed {
            /// This event's serial number.
            serial: u32,
        },
        /// Device paused notification.
        ///
        /// Notification that the device has been paused by the EIS implementation
        /// and no further events will be accepted on this device until
        /// it is resumed again.
        ///
        /// For devices of `ei_device_setup.context_type` sender, the client thus does
        /// not need to request `ei_device.stop_emulating` and may request
        /// `ei_device.start_emulating` after a subsequent `ei_device.resumed`.
        ///
        /// For devices of `ei_device_setup.context_type` receiver and where
        /// the EIS implementation did not send a `ei_device.stop_emulating`
        /// prior to this event, the device may send a `ei_device.start_emulating`
        /// event after a subsequent `ei_device.resumed` event.
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
        /// A newly advertised device is in the `ei_device.paused` state.
        Paused {
            /// This event's serial number.
            serial: u32,
        },
        /// Device start emulating event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_device.start_emulating` request for details.
        ///
        /// It is a protocol violation to send this event for a client
        /// of an `ei_handshake.context_type` other than receiver.
        StartEmulating {
            /// This event's serial number.
            serial: u32,
            /// .
            sequence: u32,
        },
        /// Device stop emulating event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_device.stop_emulating` request for details.
        ///
        /// It is a protocol violation to send this event for a client
        /// of an `ei_handshake.context_type` other than receiver.
        StopEmulating {
            /// This event's serial number.
            serial: u32,
        },
        /// Device frame event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_device.frame` request for details.
        ///
        /// It is a protocol violation to send this event for a client
        /// of an `ei_handshake.context_type` other than receiver.
        Frame {
            /// This event's serial number.
            serial: u32,
            /// Timestamp in microseconds.
            timestamp: u64,
        },
        /// Region id notification.
        ///
        /// Notifies the client that the region specified in the next `ei_device.region`
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
        /// the corresponding `ei_device.region` event. Where a device has multiple regions,
        /// this event may be sent zero or one time for each region.
        /// It is a protocol violation to send this event after the `ei_device.done` event or
        /// to send this event without a corresponding following `ei_device.region` event.
        RegionMappingId {
            /// Region mapping id.
            mapping_id: String,
        },
    }

    impl Event {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("destroyed"),
                1 => Some("name"),
                2 => Some("device_type"),
                3 => Some("dimensions"),
                4 => Some("region"),
                5 => Some("interface"),
                6 => Some("done"),
                7 => Some("resumed"),
                8 => Some("paused"),
                9 => Some("start_emulating"),
                10 => Some("stop_emulating"),
                11 => Some("frame"),
                12 => Some("region_mapping_id"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::Destroyed { serial })
                }
                1 => {
                    let name = _bytes.read_arg()?;

                    Ok(Self::Name { name })
                }
                2 => {
                    let device_type = _bytes.read_arg()?;

                    Ok(Self::DeviceType { device_type })
                }
                3 => {
                    let width = _bytes.read_arg()?;
                    let height = _bytes.read_arg()?;

                    Ok(Self::Dimensions { width, height })
                }
                4 => {
                    let offset_x = _bytes.read_arg()?;
                    let offset_y = _bytes.read_arg()?;
                    let width = _bytes.read_arg()?;
                    let hight = _bytes.read_arg()?;
                    let scale = _bytes.read_arg()?;

                    Ok(Self::Region {
                        offset_x,
                        offset_y,
                        width,
                        hight,
                        scale,
                    })
                }
                5 => {
                    let object = _bytes.read_arg()?;
                    let interface_name = _bytes.read_arg()?;
                    let version = _bytes.read_arg()?;

                    Ok(Self::Interface {
                        object: _bytes.backend().new_peer_object(
                            object,
                            interface_name,
                            version,
                        )?,
                    })
                }
                6 => Ok(Self::Done),
                7 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::Resumed { serial })
                }
                8 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::Paused { serial })
                }
                9 => {
                    let serial = _bytes.read_arg()?;
                    let sequence = _bytes.read_arg()?;

                    Ok(Self::StartEmulating { serial, sequence })
                }
                10 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::StopEmulating { serial })
                }
                11 => {
                    let serial = _bytes.read_arg()?;
                    let timestamp = _bytes.read_arg()?;

                    Ok(Self::Frame { serial, timestamp })
                }
                12 => {
                    let mapping_id = _bytes.read_arg()?;

                    Ok(Self::RegionMappingId { mapping_id })
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
                Self::Destroyed { serial } => {
                    args.push(serial.as_arg());
                }
                Self::Name { name } => {
                    args.push(name.as_arg());
                }
                Self::DeviceType { device_type } => {
                    args.push(device_type.as_arg());
                }
                Self::Dimensions { width, height } => {
                    args.push(width.as_arg());
                    args.push(height.as_arg());
                }
                Self::Region {
                    offset_x,
                    offset_y,
                    width,
                    hight,
                    scale,
                } => {
                    args.push(offset_x.as_arg());
                    args.push(offset_y.as_arg());
                    args.push(width.as_arg());
                    args.push(hight.as_arg());
                    args.push(scale.as_arg());
                }
                Self::Interface { object } => {
                    args.push(object.as_arg());
                }
                Self::Done => {}
                Self::Resumed { serial } => {
                    args.push(serial.as_arg());
                }
                Self::Paused { serial } => {
                    args.push(serial.as_arg());
                }
                Self::StartEmulating { serial, sequence } => {
                    args.push(serial.as_arg());
                    args.push(sequence.as_arg());
                }
                Self::StopEmulating { serial } => {
                    args.push(serial.as_arg());
                }
                Self::Frame { serial, timestamp } => {
                    args.push(serial.as_arg());
                    args.push(timestamp.as_arg());
                }
                Self::RegionMappingId { mapping_id } => {
                    args.push(mapping_id.as_arg());
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
/// Client-side protocol definition module for interface `ei_pointer`.
///
/**
Interface for relative pointer motion requests and events.

This interface is only provided once per device and where a client
requests `ei_pointer.release` the interface does not get re-initialized. An
EIS implementation may adjust the behavior of the device (including removing
the device) if the interface is released.
 */
pub mod pointer {
    use crate::wire;

    /// Device sub-interface for relative pointer motion.
    ///
    /// Client-side interface proxy for interface `ei_pointer`.
    ///
    /**
    Interface for relative pointer motion requests and events.

    This interface is only provided once per device and where a client
    requests `ei_pointer.release` the interface does not get re-initialized. An
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Pointer {}

    impl Pointer {
        /// Pointer sub-interface removal request.
        ///
        /// Notification that the client is no longer interested in this pointer.
        /// The EIS implementation will release any resources related to this pointer and
        /// send the `ei_pointer.destroyed` event once complete.
        pub fn release(&self) -> () {
            let args = &[];

            self.0.request(0, args);

            ()
        }

        /// Relative motion request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Generate a relative motion event on this pointer.
        ///
        /// It is a client bug to send this request more than once
        /// within the same `ei_device.frame` and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than sender.
        /// # Parameters
        ///
        /// - `x`: The x movement in logical pixels.
        /// - `y`: The y movement in logical pixels.
        ///
        pub fn motion_relative(&self, x: f32, y: f32) -> () {
            let args = &[wire::Arg::Float(x.into()), wire::Arg::Float(y.into())];

            self.0.request(1, args);

            ()
        }
    }

    /// All events of interface `ei_pointer`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
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
        Destroyed {
            /// This event's serial number.
            serial: u32,
        },
        /// Relative motion event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_pointer.motion_relative` request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than receiver.
        MotionRelative {
            /// .
            x: f32,
            /// .
            y: f32,
        },
    }

    impl Event {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("destroyed"),
                1 => Some("motion_relative"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::Destroyed { serial })
                }
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
                Self::Destroyed { serial } => {
                    args.push(serial.as_arg());
                }
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
/// Client-side protocol definition module for interface `ei_pointer_absolute`.
///
/**
Interface for absolute pointer motion.

This interface is only provided once per device and where a client
requests `ei_pointer_absolute.release` the interface does not get
re-initialized. An EIS implementation may adjust the behavior of the
device (including removing the device) if the interface is released.
 */
pub mod pointer_absolute {
    use crate::wire;

    /// Device sub-interface for absolute pointer motion.
    ///
    /// Client-side interface proxy for interface `ei_pointer_absolute`.
    ///
    /**
    Interface for absolute pointer motion.

    This interface is only provided once per device and where a client
    requests `ei_pointer_absolute.release` the interface does not get
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for PointerAbsolute {}

    impl PointerAbsolute {
        /// Absolute pointer sub-interface removal request.
        ///
        /// Notification that the client is no longer interested in this object.
        /// The EIS implementation will release any resources related to this object and
        /// send the `ei_pointer_absolute.destroyed` event once complete.
        pub fn release(&self) -> () {
            let args = &[];

            self.0.request(0, args);

            ()
        }

        /// Absolute motion request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Generate an absolute motion event on this pointer. The x/y
        /// coordinates must be within the device's regions or the event
        /// is silently discarded.
        ///
        /// It is a client bug to send this request more than once
        /// within the same `ei_device.frame` and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than sender.
        /// # Parameters
        ///
        /// - `x`: The x position in logical pixels.
        /// - `y`: The y position in logical pixels.
        ///
        pub fn motion_absolute(&self, x: f32, y: f32) -> () {
            let args = &[wire::Arg::Float(x.into()), wire::Arg::Float(y.into())];

            self.0.request(1, args);

            ()
        }
    }

    /// All events of interface `ei_pointer_absolute`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
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
        Destroyed {
            /// This event's serial number.
            serial: u32,
        },
        /// Absolute motion event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_pointer_absolute.motion_absolute` request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than receiver.
        MotionAbsolute {
            /// .
            x: f32,
            /// .
            y: f32,
        },
    }

    impl Event {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("destroyed"),
                1 => Some("motion_absolute"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::Destroyed { serial })
                }
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
                Self::Destroyed { serial } => {
                    args.push(serial.as_arg());
                }
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
/// Client-side protocol definition module for interface `ei_scroll`.
///
/**
Interface for scroll requests and events.

This interface is only provided once per device and where a client
requests `ei_scroll.release` the interface does not get
re-initialized. An EIS implementation may adjust the behavior of the
device (including removing the device) if the interface is released.
 */
pub mod scroll {
    use crate::wire;

    /// Scroll object.
    ///
    /// Client-side interface proxy for interface `ei_scroll`.
    ///
    /**
    Interface for scroll requests and events.

    This interface is only provided once per device and where a client
    requests `ei_scroll.release` the interface does not get
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Scroll {}

    impl Scroll {
        /// Scroll removal request.
        ///
        /// Notification that the client is no longer interested in this object.
        /// The EIS implementation will release any resources related to this object and
        /// send the `ei_scroll.destroyed` event once complete.
        pub fn release(&self) -> () {
            let args = &[];

            self.0.request(0, args);

            ()
        }

        /// Scroll request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Generate a a smooth (pixel-precise) scroll event on this pointer.
        /// Clients must not send `ei_scroll.scroll_discrete` events for the same event,
        /// the EIS implementation is responsible for emulation of discrete
        /// scroll events.
        ///
        /// It is a client bug to send this request more than once
        /// within the same `ei_device.frame` and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than sender.
        /// # Parameters
        ///
        /// - `x`: The x movement in logical pixels.
        /// - `y`: The y movement in logical pixels.
        ///
        pub fn scroll(&self, x: f32, y: f32) -> () {
            let args = &[wire::Arg::Float(x.into()), wire::Arg::Float(y.into())];

            self.0.request(1, args);

            ()
        }

        /// Scroll discrete request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Generate a a discrete (e.g. wheel) scroll event on this pointer.
        /// Clients must not send `ei_scroll.scroll` events for the same event,
        /// the EIS implementation is responsible for emulation of smooth
        /// scroll events.
        ///
        /// A discrete scroll event is based logical scroll units (equivalent to one
        /// mouse wheel click). The value for one scroll unit is 120, a fraction or
        /// multiple thereof represents a fraction or multiple of a wheel click.
        ///
        /// It is a client bug to send this request more than once
        /// within the same `ei_device.frame` and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than sender.
        /// # Parameters
        ///
        /// - `x`: The x movement in fractions or multiples of 120.
        /// - `y`: The y movement in fractions or multiples of 120.
        ///
        pub fn scroll_discrete(&self, x: i32, y: i32) -> () {
            let args = &[wire::Arg::Int32(x.into()), wire::Arg::Int32(y.into())];

            self.0.request(2, args);

            ()
        }

        /// Scroll stop request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Generate a a scroll stop or cancel event on this pointer.
        ///
        /// A scroll stop event notifies the EIS implementation that the interaction causing a
        /// scroll motion previously triggered with `ei_scroll.scroll` or
        /// `ei_scroll.scroll_discrete` has stopped. For example, if all
        /// fingers are lifted off a touchpad, two-finger scrolling has logically
        /// stopped. The EIS implementation may use this information to e.g. start kinetic scrolling
        /// previously based on the previous finger speed.
        ///
        /// If is_cancel is nonzero, the event represents a cancellation of the
        /// current interaction. This indicates that the interaction has stopped to the
        /// point where further (server-emulated) scroll events from this device are wrong.
        ///
        /// It is a client bug to send this request more than once
        /// within the same `ei_device.frame` and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a client bug to send this request for an axis that
        /// had a a nonzero value in either `ei_scroll.scroll` or `ei_scroll.scroll_discrete`
        /// in the current frame and the EIS implementation
        /// may ignore either or all such requests and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than sender.
        /// # Parameters
        ///
        /// - `x`: Nonzero if this axis stopped scrolling.
        /// - `y`: Nonzero if this axis stopped scrolling.
        /// - `is_cancel`: Nonzero to indicate this is a cancel event.
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

    /// All events of interface `ei_scroll`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
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
        Destroyed {
            /// This event's serial number.
            serial: u32,
        },
        /// Scroll event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_scroll.scroll` request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than receiver.
        Scroll {
            /// .
            x: f32,
            /// .
            y: f32,
        },
        /// Discrete scroll event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_scroll.scroll_discrete` request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than receiver.
        ScrollDiscrete {
            /// .
            x: i32,
            /// .
            y: i32,
        },
        /// Scroll stop event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_scroll.scroll_stop` request for details.
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than receiver.
        ScrollStop {
            /// .
            x: u32,
            /// .
            y: u32,
            /// .
            is_cancel: u32,
        },
    }

    impl Event {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("destroyed"),
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
                0 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::Destroyed { serial })
                }
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
                Self::Destroyed { serial } => {
                    args.push(serial.as_arg());
                }
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
/// Client-side protocol definition module for interface `ei_button`.
///
/**
Interface for button requests and events.

This interface is only provided once per device and where a client
requests `ei_button.release` the interface does not get
re-initialized. An EIS implementation may adjust the behavior of the
device (including removing the device) if the interface is released.
 */
pub mod button {
    use crate::wire;

    /// Button object.
    ///
    /// Client-side interface proxy for interface `ei_button`.
    ///
    /**
    Interface for button requests and events.

    This interface is only provided once per device and where a client
    requests `ei_button.release` the interface does not get
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Button {}

    impl Button {
        /// Button removal request.
        ///
        /// Notification that the client is no longer interested in this object.
        /// The EIS implementation will release any resources related to this object and
        /// send the `ei_button.destroyed` event once complete.
        pub fn release(&self) -> () {
            let args = &[];

            self.0.request(0, args);

            ()
        }

        /// Button state change request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Generate a button event on this pointer.
        ///
        /// The button codes must match the defines in linux/input-event-codes.h.
        ///
        /// It is a client bug to send more than one button request for the same button
        /// within the same `ei_device.frame` and the EIS implementation
        /// may ignore either or all button state changes and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than sender.
        /// # Parameters
        ///
        /// - `button`: Button code.
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

    /// All events of interface `ei_button`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /// Pointer removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This pointer has been removed and a client should release all
        /// associated resources.
        ///
        /// This `ei_scroll` object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        Destroyed {
            /// This event's serial number.
            serial: u32,
        },
        /// Button state change event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_scroll.button` request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than receiver.
        ///
        /// It is an EIS implementation bug to send more than one button request
        /// for the same button within the same `ei_device.frame`.
        Button {
            /// .
            button: u32,
            /// .
            state: ButtonState,
        },
    }

    impl Event {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("destroyed"),
                1 => Some("button"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::Destroyed { serial })
                }
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
                Self::Destroyed { serial } => {
                    args.push(serial.as_arg());
                }
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
/// Client-side protocol definition module for interface `ei_keyboard`.
///
/**
Interface for keyboard requests and events.

This interface is only provided once per device and where a client
requests `ei_keyboard.release` the interface does not get re-initialized. An
EIS implementation may adjust the behavior of the device (including removing
the device) if the interface is released.
 */
pub mod keyboard {
    use crate::wire;

    /// Keyboard object.
    ///
    /// Client-side interface proxy for interface `ei_keyboard`.
    ///
    /**
    Interface for keyboard requests and events.

    This interface is only provided once per device and where a client
    requests `ei_keyboard.release` the interface does not get re-initialized. An
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Keyboard {}

    impl Keyboard {
        /// Keyboard removal request.
        ///
        /// Notification that the client is no longer interested in this keyboard.
        /// The EIS implementation will release any resources related to this keyboard and
        /// send the `ei_keyboard.destroyed` event once complete.
        pub fn release(&self) -> () {
            let args = &[];

            self.0.request(0, args);

            ()
        }

        /// Key state change request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Generate a key event on this keyboard. If the device has an
        /// `ei_keyboard.keymap`, the key code corresponds to that keymap.
        ///
        /// The key codes must match the defines in linux/input-event-codes.h.
        ///
        /// It is a client bug to send more than one key request for the same key
        /// within the same `ei_device.frame` and the EIS implementation
        /// may ignore either or all key state changes and/or disconnect the client.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than sender.
        /// # Parameters
        ///
        /// - `key`: The key code.
        /// - `state`: Logical state of the key.
        ///
        pub fn key(&self, key: u32, state: KeyState) -> () {
            let args = &[
                wire::Arg::Uint32(key.into()),
                wire::Arg::Uint32(state.into()),
            ];

            self.0.request(1, args);

            ()
        }
    }

    pub use crate::eiproto_enum::keyboard::KeyState;
    pub use crate::eiproto_enum::keyboard::KeymapType;

    /// All events of interface `ei_keyboard`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /// Keyboard removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This keyboard has been removed and a client should release all
        /// associated resources.
        ///
        /// This `ei_keyboard` object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        Destroyed {
            /// This event's serial number.
            serial: u32,
        },
        /// Keymap notification.
        ///
        /// Notification that this device has a keymap. Future key events must be
        /// interpreted by the client according to this keymap. For clients
        /// of `ei_handshake.context_type` sender it is the client's
        /// responsibility to send the correct `ei_keyboard.key` keycodes to
        /// generate the expected keysym in the EIS implementation.
        ///
        /// The keymap is constant for the lifetime of the device.
        ///
        /// This event provides a file descriptor to the client that can be
        /// memory-mapped in read-only mode to provide a keyboard mapping
        /// description. The fd must be mapped with MAP_PRIVATE by
        /// the recipient, as MAP_SHARED may fail.
        ///
        /// This event is optional and only sent immediately after the `ei_keyboard` object is created
        /// and before the `ei_device.done` event. It is a protocol violation to send this
        /// event after the `ei_device.done` event.
        Keymap {
            /// The keymap type.
            keymap_type: KeymapType,
            /// The keymap size in bytes.
            size: u32,
            /// File descriptor to the keymap.
            keymap: std::os::unix::io::OwnedFd,
        },
        /// Key state change event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_keyboard.key` request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than receiver.
        ///
        /// It is a protocol violation to send a key down event in the same
        /// frame as a key up event for the same key in the same frame.
        Key {
            /// .
            key: u32,
            /// .
            state: KeyState,
        },
        /// Modifier change event.
        ///
        /// Notification that the EIS implementation has changed group or modifier
        /// states on this device, but not necessarily in response to an
        /// `ei_keyboard.key` event or request. Future `ei_keyboard.key` requests must
        /// take the new group and modifier state into account.
        ///
        /// This event should be sent any time the modifier state or effective group
        /// has changed, whether caused by an `ei_keyboard.key` event in accordance
        /// with the keymap, indirectly due to further handling of an
        /// `ei_keyboard.key` event (e.g., because it triggered a keyboard shortcut
        /// that then changed the state), or caused by an unrelated an event (e.g.,
        /// input from a different keyboard, or a group change triggered by a layout
        /// selection widget).
        ///
        /// For receiver clients, modifiers events will always be properly ordered
        /// with received key events, so each key event should be interpreted using
        /// the most recently-received modifier state. The EIS implementation should
        /// send this event immediately following the `ei_device.frame` event for the
        /// key press that caused the change. If the state change impacts multiple
        /// keyboards, this event should be sent for all of them.
        ///
        /// For sender clients, the modifiers event is not inherently synchronized
        /// with key requests, but the client may send an `ei_connection.sync` request
        /// when synchronization is required. When the corresponding
        /// `ei_callback.done` event is received, all key requests sent prior to the
        /// sync request are guaranteed to have been processed, and any
        /// directly-resulting modifiers events are guaranteed to have been
        /// received. Note, however, that it is still possible for
        /// indirectly-triggered state changes, such as via a keyboard shortcut not
        /// encoded in the keymap, to be reported after the done event.
        ///
        /// A client must assume that all modifiers are lifted when it
        /// receives an `ei_device.paused` event. The EIS implementation
        /// must send this event after `ei_device.resumed` to notify the client
        /// of any nonzero modifier state.
        ///
        /// This event does not require an `ei_device.frame` and should
        /// be processed immediately by the client.
        ///
        /// This event is only sent for devices with an `ei_keyboard.keymap`.
        ///
        /// Note: A previous version of the documentation instead specified that
        /// this event should not be sent in response to `ei_keyboard.key` events that
        /// change the group or modifier state according to the keymap. However,
        /// this complicated client implementation and resulted in situations where
        /// the client state could get out of sync with the EIS implementation.
        Modifiers {
            /// This event's serial number.
            serial: u32,
            /// Depressed modifiers.
            depressed: u32,
            /// Locked modifiers.
            locked: u32,
            /// Latched modifiers.
            latched: u32,
            /// The keyboard group (layout).
            group: u32,
        },
    }

    impl Event {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("destroyed"),
                1 => Some("keymap"),
                2 => Some("key"),
                3 => Some("modifiers"),
                _ => None,
            }
        }

        pub(super) fn parse(
            operand: u32,
            _bytes: &mut wire::ByteStream,
        ) -> Result<Self, wire::ParseError> {
            match operand {
                0 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::Destroyed { serial })
                }
                1 => {
                    let keymap_type = _bytes.read_arg()?;
                    let size = _bytes.read_arg()?;
                    let keymap = _bytes.read_arg()?;

                    Ok(Self::Keymap {
                        keymap_type,
                        size,
                        keymap,
                    })
                }
                2 => {
                    let key = _bytes.read_arg()?;
                    let state = _bytes.read_arg()?;

                    Ok(Self::Key { key, state })
                }
                3 => {
                    let serial = _bytes.read_arg()?;
                    let depressed = _bytes.read_arg()?;
                    let locked = _bytes.read_arg()?;
                    let latched = _bytes.read_arg()?;
                    let group = _bytes.read_arg()?;

                    Ok(Self::Modifiers {
                        serial,
                        depressed,
                        locked,
                        latched,
                        group,
                    })
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
                Self::Destroyed { serial } => {
                    args.push(serial.as_arg());
                }
                Self::Keymap {
                    keymap_type,
                    size,
                    keymap,
                } => {
                    args.push(keymap_type.as_arg());
                    args.push(size.as_arg());
                    args.push(keymap.as_arg());
                }
                Self::Key { key, state } => {
                    args.push(key.as_arg());
                    args.push(state.as_arg());
                }
                Self::Modifiers {
                    serial,
                    depressed,
                    locked,
                    latched,
                    group,
                } => {
                    args.push(serial.as_arg());
                    args.push(depressed.as_arg());
                    args.push(locked.as_arg());
                    args.push(latched.as_arg());
                    args.push(group.as_arg());
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
/// Client-side protocol definition module for interface `ei_touchscreen`.
///
/**
Interface for touchscreen requests and events.

This interface is only provided once per device and where a client
requests `ei_touchscreen.release` the interface does not get re-initialized. An
EIS implementation may adjust the behavior of the device (including removing
the device) if the interface is released.
 */
pub mod touchscreen {
    use crate::wire;

    /// Touchscreen object.
    ///
    /// Client-side interface proxy for interface `ei_touchscreen`.
    ///
    /**
    Interface for touchscreen requests and events.

    This interface is only provided once per device and where a client
    requests `ei_touchscreen.release` the interface does not get re-initialized. An
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
        const CLIENT_SIDE: bool = true;

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

    impl crate::ei::Interface for Touchscreen {}

    impl Touchscreen {
        /// Touch removal request.
        ///
        /// Notification that the client is no longer interested in this touchscreen.
        /// The EIS implementation will release any resources related to this touch and
        /// send the `ei_touchscreen.destroyed` event once complete.
        pub fn release(&self) -> () {
            let args = &[];

            self.0.request(0, args);

            ()
        }

        /// Touch down request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Notifies the EIS implementation about a new touch logically down at the
        /// given coordinates. The touchid is a unique id for this touch. Touchids
        /// may be re-used after `ei_touchscreen.up`.
        ///
        /// The x/y coordinates must be within the device's regions or the event and future
        /// `ei_touchscreen.motion` events with the same touchid are silently discarded.
        ///
        /// It is a protocol violation to send a touch down in the same
        /// frame as a touch motion or touch up.
        /// # Parameters
        ///
        /// - `touchid`: A unique touch id to identify this touch.
        /// - `x`: Touch x coordinate in logical pixels.
        /// - `y`: Touch y coordinate in logical pixels.
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

        /// Touch motion request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Notifies the EIS implementation about an existing touch changing position to
        /// the given coordinates. The touchid is the unique id for this touch previously
        /// sent with `ei_touchscreen.down`.
        ///
        /// The x/y coordinates must be within the device's regions or the event is
        /// silently discarded.
        ///
        /// It is a protocol violation to send a touch motion in the same
        /// frame as a touch down or touch up.
        /// # Parameters
        ///
        /// - `touchid`: A unique touch id to identify this touch.
        /// - `x`: Touch x coordinate in logical pixels.
        /// - `y`: Touch y coordinate in logical pixels.
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

        /// Touch up request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Notifies the EIS implementation about an existing touch being logically
        /// up. The touchid is the unique id for this touch previously
        /// sent with `ei_touchscreen.down`.
        ///
        /// If a touch is cancelled via `ei_touchscreen.cancel`, the `ei_touchscreen.up`
        /// request must not be sent for this same touch. Likewise, a touch released
        /// with `ei_touchscreen.up` must not be cancelled.
        ///
        /// The touchid may be re-used after this request.
        ///
        /// It is a protocol violation to send a touch up in the same
        /// frame as a touch motion or touch down.
        /// # Parameters
        ///
        /// - `touchid`: A unique touch id to identify this touch.
        ///
        pub fn up(&self, touchid: u32) -> () {
            let args = &[wire::Arg::Uint32(touchid.into())];

            self.0.request(3, args);

            ()
        }

        /// Touch cancel request.
        ///
        /// **Note:** This request may only be used in a sender [context type](crate::ei::handshake::ContextType).
        ///
        /// Notifies the EIS implementation about an existing touch being cancelled.
        /// This typically means that any effects the touch may have had on the
        /// user interface should be reverted or otherwise made inconsequential.
        ///
        /// This request replaces `ei_touchscreen.up` for the same touch.
        /// If a touch is cancelled via `ei_touchscreen.cancel`, the `ei_touchscreen.up`
        /// request must not be sent for this same touch. Likewise, a touch released
        /// with `ei_touchscreen.up` must not be cancelled.
        ///
        /// The touchid is the unique id for this touch previously
        /// sent with `ei_touchscreen.down`.
        ///
        /// The touchid may be re-used after this request.
        ///
        /// It is a protocol violation to send a touch cancel
        /// in the same frame as a touch motion or touch down.
        /// # Parameters
        ///
        /// - `touchid`: A unique touch id to identify this touch.
        ///
        pub fn cancel(&self, touchid: u32) -> () {
            let args = &[wire::Arg::Uint32(touchid.into())];

            self.0.request(4, args);

            ()
        }
    }

    /// All events of interface `ei_touchscreen`.
    ///
    /// Events are messages that come from servers.
    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /// Touchscreen removal notification.
        ///
        /// **Note:** This event is a destructor.
        ///
        /// This touch has been removed and a client should release all
        /// associated resources.
        ///
        /// This `ei_touchscreen` object will be destroyed by the EIS implementation immediately after
        /// after this event is sent and as such the client must not attempt to use
        /// it after that point.
        Destroyed {
            /// This event's serial number.
            serial: u32,
        },
        /// Touch down event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_touchscreen.down` request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than receiver.
        ///
        /// It is a protocol violation to send a touch down in the same
        /// frame as a touch motion or touch up.
        Down {
            /// .
            touchid: u32,
            /// .
            x: f32,
            /// .
            y: f32,
        },
        /// Touch motion event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_touchscreen.motion` request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than receiver.
        ///
        /// It is a protocol violation to send a touch motion in the same
        /// frame as a touch down or touch up.
        Motion {
            /// .
            touchid: u32,
            /// .
            x: f32,
            /// .
            y: f32,
        },
        /// Touch motion event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_touchscreen.up` request for details.
        ///
        /// It is a protocol violation to send this request for a client
        /// of an `ei_handshake.context_type` other than receiver.
        ///
        /// If a touch is released via `ei_touchscreen.up`, no `ei_touchscreen.cancel`
        /// event is sent for this same touch. Likewise, a touch released
        /// with `ei_touchscreen.cancel` must not be released via `ei_touchscreen.up`.
        ///
        /// It is a protocol violation to send a touch up in the same
        /// frame as a touch motion or touch down.
        Up {
            /// .
            touchid: u32,
        },
        /// Touch cancel event.
        ///
        /// **Note:** This event may only be used in a receiver [context type](crate::ei::handshake::ContextType).
        ///
        /// See the `ei_touchscreen.cancel` request for details.
        ///
        /// It is a protocol violation to send this event for a client
        /// of an `ei_handshake.context_type` other than receiver.
        ///
        /// If a touch is cancelled via `ei_touchscreen.cancel`, no `ei_touchscreen.up`
        /// event is sent for this same touch. Likewise, a touch released
        /// with `ei_touchscreen.up` must not be cancelled via `ei_touchscreen.cancel`.
        ///
        /// It is a protocol violation to send a touch cancel event in the same
        /// frame as a touch motion or touch down.
        Cancel {
            /// .
            touchid: u32,
        },
    }

    impl Event {
        pub(super) fn op_name(operand: u32) -> Option<&'static str> {
            match operand {
                0 => Some("destroyed"),
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
                0 => {
                    let serial = _bytes.read_arg()?;

                    Ok(Self::Destroyed { serial })
                }
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
                Self::Destroyed { serial } => {
                    args.push(serial.as_arg());
                }
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

/// All events of all interfaces.
///
/// Events are messages that come from servers.
#[non_exhaustive]
#[derive(Debug)]
pub enum Event {
    Handshake(handshake::Handshake, handshake::Event),
    Connection(connection::Connection, connection::Event),
    Callback(callback::Callback, callback::Event),
    Pingpong(pingpong::Pingpong, pingpong::Event),
    Seat(seat::Seat, seat::Event),
    Device(device::Device, device::Event),
    Pointer(pointer::Pointer, pointer::Event),
    PointerAbsolute(pointer_absolute::PointerAbsolute, pointer_absolute::Event),
    Scroll(scroll::Scroll, scroll::Event),
    Button(button::Button, button::Event),
    Keyboard(keyboard::Keyboard, keyboard::Event),
    Touchscreen(touchscreen::Touchscreen, touchscreen::Event),
}

impl Event {
    pub(crate) fn op_name(interface: &str, operand: u32) -> Option<&'static str> {
        match interface {
            "ei_handshake" => handshake::Event::op_name(operand),
            "ei_connection" => connection::Event::op_name(operand),
            "ei_callback" => callback::Event::op_name(operand),
            "ei_pingpong" => pingpong::Event::op_name(operand),
            "ei_seat" => seat::Event::op_name(operand),
            "ei_device" => device::Event::op_name(operand),
            "ei_pointer" => pointer::Event::op_name(operand),
            "ei_pointer_absolute" => pointer_absolute::Event::op_name(operand),
            "ei_scroll" => scroll::Event::op_name(operand),
            "ei_button" => button::Event::op_name(operand),
            "ei_keyboard" => keyboard::Event::op_name(operand),
            "ei_touchscreen" => touchscreen::Event::op_name(operand),
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
                handshake::Event::parse(operand, bytes)?,
            )),
            "ei_connection" => Ok(Self::Connection(
                object.downcast_unchecked(),
                connection::Event::parse(operand, bytes)?,
            )),
            "ei_callback" => Ok(Self::Callback(
                object.downcast_unchecked(),
                callback::Event::parse(operand, bytes)?,
            )),
            "ei_pingpong" => Ok(Self::Pingpong(
                object.downcast_unchecked(),
                pingpong::Event::parse(operand, bytes)?,
            )),
            "ei_seat" => Ok(Self::Seat(
                object.downcast_unchecked(),
                seat::Event::parse(operand, bytes)?,
            )),
            "ei_device" => Ok(Self::Device(
                object.downcast_unchecked(),
                device::Event::parse(operand, bytes)?,
            )),
            "ei_pointer" => Ok(Self::Pointer(
                object.downcast_unchecked(),
                pointer::Event::parse(operand, bytes)?,
            )),
            "ei_pointer_absolute" => Ok(Self::PointerAbsolute(
                object.downcast_unchecked(),
                pointer_absolute::Event::parse(operand, bytes)?,
            )),
            "ei_scroll" => Ok(Self::Scroll(
                object.downcast_unchecked(),
                scroll::Event::parse(operand, bytes)?,
            )),
            "ei_button" => Ok(Self::Button(
                object.downcast_unchecked(),
                button::Event::parse(operand, bytes)?,
            )),
            "ei_keyboard" => Ok(Self::Keyboard(
                object.downcast_unchecked(),
                keyboard::Event::parse(operand, bytes)?,
            )),
            "ei_touchscreen" => Ok(Self::Touchscreen(
                object.downcast_unchecked(),
                touchscreen::Event::parse(operand, bytes)?,
            )),
            intr => Err(wire::ParseError::InvalidInterface(intr.to_owned())),
        }
    }
}

impl wire::MessageEnum for Event {
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
