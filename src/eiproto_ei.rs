// GENERATED FILE

// Define module for each interface?
// TODO Cleaner/safer way to strip out Ei/Eis prefix?

/**
This is a special interface to setup the client as seen by the EIS
implementation. The object for this interface has the fixed object
id 0 and only exists until the connection has been set up, see the
`ei_handshake.connection` event.

The `ei_handshake` version is 1 until:
- the EIS implementation sends the interface_version event with
  a version other than 1, and, in response,
- the client sends the interface_version request with a
  version equal or lower to the EIS implementation version.

The EIS implementation must send the interface_version event immediately
once the physical connection has been established.

Once the `ei_connection.connection` event has been sent the handshake
is destroyed by the EIS implementation.
 */
pub mod handshake {
    #[derive(Clone, Debug)]
    pub struct Handshake {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Handshake {
        const NAME: &'static str = "ei_handshake";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Handshake {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Handshake {
        /**
        Notifies the EIS implementation that this client supports the
        given version of the `ei_handshake` interface. The version number
        must be less than or equal to the version in the
        handshake_version event sent by the EIS implementation when
        the connection was established.

        Immediately after sending this request, the client must assume the negotiated
        version number for the `ei_handshake` interface and the EIS implementation
        may send events and process requests matching that version.

        This request must be sent exactly once and it must be the first request
        the client sends.
         */
        pub fn handshake_version(&self, version: u32) -> rustix::io::Result<()> {
            let args = &[crate::Arg::Uint32(version.into())];

            self.connection.request(self.id, 0, args)?;

            Ok(())
        }

        /**
        Notify the EIS implementation that configuration is complete.

        In the future (and possibly after requiring user interaction),
        the EIS implementation responds by sending the `ei_handshake.connection` event.
         */
        pub fn finish(&self) -> rustix::io::Result<()> {
            let args = &[];

            self.connection.request(self.id, 1, args)?;

            Ok(())
        }

        /**
        Notify the EIS implementation of the type of this context. The context types
        defines whether the client will send events to or receive events from the
        EIS implementation.

        Depending on the context type, certain requests must not be used and some
        events must not be sent by the EIS implementation.

        This request is optional, the default client type is context_type.receiver.
        This request must not be sent more than once and must be sent before
        `ei_handshake.finish.`
         */
        pub fn context_type(&self, context_type: ContextType) -> rustix::io::Result<()> {
            let args = &[crate::Arg::Uint32(context_type.into())];

            self.connection.request(self.id, 2, args)?;

            Ok(())
        }

        /**
        Notify the EIS implementation of the client name. The name is a
        human-presentable UTF-8 string and should represent the client name
        as accurately as possible. This name may be presented to the user
        for identification of this client (e.g. to confirm the client has
        permissions to connect).

        There is no requirement for the EIS implementation to use this name. For
        example, where the client is managed through an XDG Desktop Portal an EIS
        implementation would typically use client identification information sent
        by the portal instead.

        This request is optional, the default client name is implementation-defined.
        This request must not be sent more than once and must be sent before
        `ei_handshake.finish.`
         */
        pub fn name(&self, name: &str) -> rustix::io::Result<()> {
            let args = &[crate::Arg::String(name.into())];

            self.connection.request(self.id, 3, args)?;

            Ok(())
        }

        /**
        Notify the EIS implementation that the client supports the
        given named interface with the given maximum version number.

        Future objects created by the EIS implementation will
        use the respective interface version (or any lesser version).

        This request must be sent for the "`ei_connection`" interface,
        failing to do so will result in the EIS implementation disconnecting
        the client on `ei_handshake.finish.`

        This request must not be sent for the "`ei_handshake`" interface, use
        the `ei_handshake.handshake_version` request instead.

        Note that an EIS implementation may consider some interfaces to
        be required and immediately `ei_connection.disconnect` a client
        not supporting those interfaces.

        This request must not be sent more than once per interface and must be
        sent before `ei_handshake.finish.`
         */
        pub fn interface_version(&self, name: &str, version: u32) -> rustix::io::Result<()> {
            let args = &[
                crate::Arg::String(name.into()),
                crate::Arg::Uint32(version.into()),
            ];

            self.connection.request(self.id, 4, args)?;

            Ok(())
        }
    }

    /**
    This enum denotes context types for the libei context.

    A context type of receiver is a libei context receiving events
    from the EIS implementation. A context type of sender is a libei context
    sending events to the EIS implementation.
     */
    #[derive(Clone, Copy, Debug)]
    pub enum ContextType {
        /** this client receives events from the EIS implementation */
        Receiver = 1,
        /** this client sends events to the EIS implementation */
        Sender = 2,
    }

    impl From<ContextType> for u32 {
        fn from(value: ContextType) -> u32 {
            value as u32
        }
    }

    impl crate::OwnedArg for ContextType {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            todo!()
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        This event is sent exactly once and immediately after connection
        to the EIS implementation.

        In response, the client must send the `ei_handshake.handshake_version` request
        with any version up to including the version provided in this event.
        See the `ei_handshake.handshake_version` request for details on what happens next.
         */
        HandshakeVersion {
            /** the interface version */
            version: u32,
        },
        /**
        Notifies the client that the EIS implementation supports
        the given named interface with the given maximum version number.

        This event must be sent by the EIS implementation for any
        interfaces that supports client-created objects (e.g. "`ei_callback`")
        before the `ei_handshake.connection` event.
        The client must not assume those interfaces are supported unless
        and until those versions have been received.

        This request must not be sent for the "`ei_handshake`" interface, use
        the handshake_version event instead.

        This event may be sent by the EIS implementation for any
        other supported interface (but not necessarily all supported
        interfaces) before the `ei_handshake.connection` event.
         */
        InterfaceVersion {
            /** the interface name */
            name: String,
            /** the interface version */
            version: u32,
        },
        /**
        Provides the client with the connection object that is the top-level
        object for all future requests and events.

        This event is sent exactly once at some unspecified time after the client
        sends the `ei_handshake.finish` request to the EIS implementation.

        The `ei_handshake` object will be destroyed by the
        EIS implementation immediately after this event has been sent, a
        client must not attempt to use it after that point.

        The version sent by the EIS implementation is the version of the "`ei_connection`"
        interface as announced by `ei_handshake.interface_version`, or any
        lower version.

        The serial number is the start value of the EIS implementation's serial
        number sequence. Clients must not assume any specific value for this
        serial number. Any future serial number in any event is monotonically
        increasing by an unspecified amount.
         */
        Connection {
            /** this event's serial number */
            serial: u32,
            /** the connection object */
            connection: super::connection::Connection,
            /** the version of the connection object */
            version: u32,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::HandshakeVersion {
                    version: bytes.read_arg()?,
                }),
                1 => Some(Self::InterfaceVersion {
                    name: bytes.read_arg()?,
                    version: bytes.read_arg()?,
                }),
                2 => Some(Self::Connection {
                    serial: bytes.read_arg()?,
                    connection: bytes.read_arg()?,
                    version: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

/**
The core connection object. This is the top-level object for any communication
with the EIS implementation.

Note that for a client to receive this object, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod connection {
    #[derive(Clone, Debug)]
    pub struct Connection {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Connection {
        const NAME: &'static str = "ei_connection";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Connection {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Connection {
        /**
        The sync request asks the EIS implementation to emit the 'done' event
        on the returned `ei_callback` object. Since requests are
        handled in-order and events are delivered in-order, this can
        be used as a synchronization point to ensure all previous requests and the
        resulting events have been handled.

        The object returned by this request will be destroyed by the
        EIS implementation after the callback is fired and as such the client must not
        attempt to use it after that point.

        The callback_data in the `ei_callback.done` event is always zero.

        Note that for a client to use this request it must announce
        support for the "`ei_callback`" interface in `ei_handshake.interface_version.`
        It is a protocol violation to request sync without having announced the
        "`ei_callback`" interface and the EIS implementation must disconnect
        the client.
         */
        pub fn sync(&self) -> rustix::io::Result<(super::callback::Callback)> {
            let callback = self.connection.new_id();
            let args = &[crate::Arg::NewId(callback.into())];

            self.connection.request(self.id, 0, args)?;

            Ok((super::callback::Callback {
                connection: self.connection.clone(),
                id: callback,
            }))
        }

        /**
        A request to the EIS implementation that this client should be disconnected.
        This is a courtesy request to allow the EIS implementation to distinquish
        between a client disconnecting on purpose and one disconnecting through the
        socket becoming invalid.

        Immediately after sending this request, the client may destroy the
        `ei_connection` object and it should close the socket. The EIS implementation
        will treat the connection as already disconnected on receipt and does not
        send the `ei_connection.disconnect` event in response to this request.
         */
        pub fn disconnect(&self) -> rustix::io::Result<()> {
            let args = &[];

            self.connection.request(self.id, 1, args)?;

            Ok(())
        }
    }

    /**
    A reason why a client was disconnected. This enum is intended to
    provide information to the client on whether it was disconnected as
    part of normal operations or as result of an error on either the client
    or EIS implementation side.

    A nonzero value describes an error, with the generic value "error" (1) reserved
    as fallback.

    This enum may be extended in the future, clients must be able to handle
    values that are not in their supported version of this enum.
     */
    #[derive(Clone, Copy, Debug)]
    pub enum DisconnectReason {
        /** client was purposely disconnected */
        Disconnected = 0,
        /** an error caused the disconnection */
        Error = 1,
        /** sender/receiver client sent request for receiver/sender mode */
        Mode = 2,
        /** client committed a protocol violation */
        Protocol = 3,
        /** client sent an invalid value */
        Value = 4,
        /** error on the transport layer */
        Transport = 5,
    }

    impl From<DisconnectReason> for u32 {
        fn from(value: DisconnectReason) -> u32 {
            value as u32
        }
    }

    impl crate::OwnedArg for DisconnectReason {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            todo!()
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        This event may be sent by the EIS implementation immediately before
        the client is disconnected. The last_serial argument is set to the last
        serial number used in a request by the client or zero if the client has not
        yet issued a request.

        Where a client is disconnected by EIS on purpose, for example after
        a user interaction, the reason is disconnect_reason.disconnected (i.e. zero)
        and the explanation is NULL.

        Where a client is disconnected due to some invalid request or other
        protocol error, the reason is one of disconnect_reason (i.e. nonzero) and
        explanation may contain a string explaining why. This string is
        intended to help debugging only and is not guaranteed to stay constant.

        The `ei_connection` object will be destroyed by the
        EIS implementation immediately after this event has been sent, a
        client must not attempt to use it after that point.

        There is no guarantee this event is sent - the connection may be closed
        without a disconnection event.
         */
        Disconnected {
            /** the last serial sent by the EIS implementation */
            last_serial: u32,
            /** the reason for being disconnected */
            reason: DisconnectReason,
            /** an explanation for debugging purposes */
            explanation: String,
        },
        /**
        Notification that a new seat has been added.

        A seat is a set of input devices that logically belong together.

        This event is only sent if the client announced support for the
        "`ei_seat`" interface in `ei_handshake.interface_version.`
        The interface version is equal or less to the client-supported
        version in `ei_handshake.interface_version` for the "`ei_seat`"
        interface.
         */
        Seat {
            /**  */
            seat: super::seat::Seat,
            /** the interface version */
            version: u32,
        },
        /**
        Notification that an object ID used in an earlier request was
        invalid and does not exist.

        This event is sent by the EIS implementation when an object that
        does not exist as seen by the EIS implementation. The protocol is
        asynchronous and this may occur e.g. when the EIS implementation
        destroys an object at the same time as the client requests functionality
        from that object. For example, an EIS implementation may send
        `ei_device.destroyed` and destroy the device's resources (and protocol object)
        at the same time as the client attempts to `ei_device.start_emulating`
        on that object.

        It is the client's responsibilty to unwind any state changes done
        to the object since the last successful message.
         */
        InvalidObject {
            /** the last serial sent by the EIS implementation */
            last_serial: u32,
            /**  */
            invalid_id: u64,
        },
        /**
        The ping event asks the client to emit the 'done' event
        on the provided `ei_callback` object. Since requests are
        handled in-order and events are delivered in-order, this can
        be used as a synchronization point to ensure all previous requests
        and the resulting events have been handled.

        The object returned by this request must be destroyed by the
        ei client implementation after the callback is fired and as
        such the client must not attempt to use it after that point.

        The callback_data in the resulting `ei_pingpong.done` request is
        ignored by the EIS implementation.

        Note that for a EIS implementation to use this request the client must
        announce support for this interface in `ei_handshake.interface_version.` It is
        a protocol violation to send this event to a client without the
        "`ei_pingpong`" interface.
         */
        Ping {
            /** callback object for the ping request */
            ping: super::pingpong::Pingpong,
            /** the version of the callback object */
            version: u32,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::Disconnected {
                    last_serial: bytes.read_arg()?,
                    reason: bytes.read_arg()?,
                    explanation: bytes.read_arg()?,
                }),
                1 => Some(Self::Seat {
                    seat: bytes.read_arg()?,
                    version: bytes.read_arg()?,
                }),
                2 => Some(Self::InvalidObject {
                    last_serial: bytes.read_arg()?,
                    invalid_id: bytes.read_arg()?,
                }),
                3 => Some(Self::Ping {
                    ping: bytes.read_arg()?,
                    version: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

/**
Interface for ensuring a roundtrip to the EIS implementation.
Clients can handle the 'done' event to get notified when
the related request that created the `ei_callback` object is done.

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod callback {
    #[derive(Clone, Debug)]
    pub struct Callback {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Callback {
        const NAME: &'static str = "ei_callback";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Callback {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Callback {}

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        Notify the client when the related request is done. Immediately after this event
        the `ei_callback` object is destroyed by the EIS implementation and as such the
        client must not attempt to use it after that point.
         */
        Done {
            /** request-specific data for the callback */
            callback_data: u64,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::Done {
                    callback_data: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

/**
Interface for ensuring a roundtrip to the client implementation.
This interface is identical to `ei_callback` but is intended for
the EIS implementation to enforce a roundtrip to the client.

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod pingpong {
    #[derive(Clone, Debug)]
    pub struct Pingpong {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Pingpong {
        const NAME: &'static str = "ei_pingpong";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Pingpong {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Pingpong {
        /**
        Notify the EIS implementation when the related event is done. Immediately after this request
        the `ei_pingpong` object is destroyed by the client and as such must not be used
        any further.
         */
        pub fn done(&self, callback_data: u64) -> rustix::io::Result<()> {
            let args = &[crate::Arg::Uint64(callback_data.into())];

            self.connection.request(self.id, 0, args)?;

            Ok(())
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {}

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                _ => None,
            }
        }
    }
}

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

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod seat {
    #[derive(Clone, Debug)]
    pub struct Seat {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Seat {
        const NAME: &'static str = "ei_seat";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Seat {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Seat {
        /**
        Notification that the client is no longer interested in this seat.
        The EIS implementation will release any resources related to this seat and
        send the `ei_seat.destroyed` event once complete.

        Note that releasing a seat does not guarantee another seat becomes available.
        In other words, in most single-seat cases, releasing the seat means the
        connection becomes effectively inert.
         */
        pub fn release(&self) -> rustix::io::Result<()> {
            let args = &[];

            self.connection.request(self.id, 0, args)?;

            Ok(())
        }

        /**
        Bind to the bitmask of capabilities given. The bitmask is zero or more of the
        masks representing an interface as provided in the `ei_seat.capability` event.
        See the `ei_seat.capability` event documentation for examples.

        Binding masks that are not supported in the `ei_device`'s interface version
        is a client bug and may result in disconnection.

        A client may send this request multiple times to adjust the capabilities it
        is interested in. If previously-bound capabilities are dropped by the client,
        the EIS implementation may `ei_device.remove` devices that have these capabilities.
         */
        pub fn bind(&self, capabilities: u64) -> rustix::io::Result<()> {
            let args = &[crate::Arg::Uint64(capabilities.into())];

            self.connection.request(self.id, 1, args)?;

            Ok(())
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        This seat has been removed and a client should release all
        associated resources.

        This `ei_seat` object will be destroyed by the EIS implementation immmediately after
        after this event is sent and as such the client must not attempt to use
        it after that point.
         */
        Destroyed {
            /** this event's serial number */
            serial: u32,
        },
        /**
        The name of this seat, if any. This event is optional and sent once immediately
        after object creation.

        It is a protocol violation to send this event after the `ei_seat.done` event.
         */
        Name {
            /** the seat name */
            name: String,
        },
        /**
        A notification that this seat supports devices with the given interface.
        The interface is mapped to a bitmask by the EIS implementation.
        A client may then binary OR these bitmasks in `ei_seat.bind.`
        In response, the EIS implementation may then create device based on those
        bound capabilities.

        For example, an EIS implementation may map "`ei_pointer`" to 0x1,
        "`ei_keyboard`" to 0x4 and "`ei_touchscreen`" to 0x8. A client may then
        `ei_seat.bind`(0xc) to bind to keyboard and touchscreen but not pointer.
        Note that as shown in this example the set of masks may be sparse.
        The value of the mask is contant for the lifetime of the seat but may differ
        between seats.

        Note that seat capabilities only represent a mask of possible capabilities on
        devices in this seat. A capability that is not available on the seat cannot
        ever be available on any device in this seat. For example, a seat that only has the
        pointer and keyboard capabilities can never have a device with the touchscreen
        capability. It is up to the EIS implementation to decide how many (if any) devices
        with any given capability exist in this seat.

        Only interfaces that the client announced during `ei_handshake.interface_version`
        can be a seat capability.

        This event is sent multiple times - once per supported interface.
        The set of capabilities is constant for the lifetime of the seat.

        It is a protocol violation to send this event after the `ei_seat.done` event.
         */
        Capability {
            /** the mask representing this capability */
            mask: u64,
            /** the interface name for this capability */
            interface: String,
        },
        /**
        Notification that the initial burst of events is complete and
        the client can set up this seat now.

        It is a protocol violation to send this event more than once.
         */
        Done,
        /**
        Notification that a new device has been added.

        This event is only sent if the client announced support for the
        "`ei_device`" interface in `ei_handshake.interface_version.`
        The interface version is equal or less to the client-supported
        version in `ei_handshake.interface_version` for the "`ei_device`"
        interface.
         */
        Device {
            /** the new device */
            device: super::device::Device,
            /** the interface version */
            version: u32,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::Destroyed {
                    serial: bytes.read_arg()?,
                }),
                1 => Some(Self::Name {
                    name: bytes.read_arg()?,
                }),
                2 => Some(Self::Capability {
                    mask: bytes.read_arg()?,
                    interface: bytes.read_arg()?,
                }),
                3 => Some(Self::Done),
                4 => Some(Self::Device {
                    device: bytes.read_arg()?,
                    version: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

/**
An `ei_device` represents a single logical input devices. Like physical input devices
an `ei_device` may have multiple capabilities and may e.g. function as pointer
and keyboard.

Depending on the `ei_handshake.context_type`, an `ei_device` can
emulate events via client requests or receive events. It is a protocol violation
to emulate certain events on a receiver device, or for the EIS implementation
to send certain events to the device. See the individual request/event documentation
for details.

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod device {
    #[derive(Clone, Debug)]
    pub struct Device {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Device {
        const NAME: &'static str = "ei_device";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Device {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Device {
        /**
        Notification that the client is no longer interested in this device.

        Note that releasing a device does not guarantee another device becomes available.

        The EIS implementation will release any resources related to this device and
        send the `ei_device.destroyed` event once complete.
         */
        pub fn release(&self) -> rustix::io::Result<()> {
            let args = &[];

            self.connection.request(self.id, 0, args)?;

            Ok(())
        }

        /**
        Notify the EIS implementation that the given device is about to start
        sending events. This should be seen more as a transactional boundary than a
        time-based boundary. The primary use-cases for this are to allow for setup on
        the EIS implementation side and/or UI updates to indicate that a device is
        sending events now and for out-of-band information to sync with a given event
        sequence.

        There is no actual requirement that events start immediately once emulation
        starts and there is no requirement that a client calls `ei_device.stop_emulating`
        after the most recent events.
        For example, in a remote desktop use-case the client would call
        `ei_device.start_emulating` once the remote desktop session starts (rather than when
        the device sends events) and `ei_device.stop_emulating` once the remote desktop
        session stops.

        The sequence number identifies this transaction between start/stop emulating.
        It must go up by at least 1 on each call to `ei_device.start_emulating.`
        Wraparound must be handled by the EIS implementation but callers must ensure
        that detection of wraparound is possible.

        It is a protocol violation to request `ei_device.start_emulating` after
        `ei_device.start_emulating` without an intermediate stop_emulating.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than sender.
         */
        pub fn start_emulating(&self, last_serial: u32, sequence: u32) -> rustix::io::Result<()> {
            let args = &[
                crate::Arg::Uint32(last_serial.into()),
                crate::Arg::Uint32(sequence.into()),
            ];

            self.connection.request(self.id, 1, args)?;

            Ok(())
        }

        /**
        Notify the EIS implementation that the given device is no longer sending
        events. See `ei_device.start_emulating` for details.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than sender.
         */
        pub fn stop_emulating(&self, last_serial: u32) -> rustix::io::Result<()> {
            let args = &[crate::Arg::Uint32(last_serial.into())];

            self.connection.request(self.id, 2, args)?;

            Ok(())
        }

        /**
        Generate a frame event to group the current set of events
        into a logical hardware event. This function must be called after one
        or more events on any of `ei_pointer`, `ei_pointer_absolute`,
        `ei_scroll`, `ei_button`, `ei_keyboard` or `ei_touchscreen` has
        been requested by the EIS implementation.

        The EIS implementation should not process changes to the device state
        until the `ei_device.frame` event. For example, pressing and releasing
        a key within the same frame is a logical noop.

        The given timestamp applies to all events in the current frame.
        The timestamp must be in microseconds of CLOCK_MONOTONIC.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than sender.
         */
        pub fn frame(&self, last_serial: u32, timestamp: u64) -> rustix::io::Result<()> {
            let args = &[
                crate::Arg::Uint32(last_serial.into()),
                crate::Arg::Uint64(timestamp.into()),
            ];

            self.connection.request(self.id, 3, args)?;

            Ok(())
        }
    }

    /**
    If the device type is `ei_device.device_type.virtual`, the device is a
    virtual device representing input as applied on the EIS implementation's
    screen. A relative virtual device generates input events in logical pixels,
    an absolute virtual device generates input events in logical pixels on one
    of the device's regions. Virtual devices do not have a `ei_device.dimension` but
    it may have an `ei_device.region.`

    If the device type is `ei_device.device_type.physical`, the device is a
    representation of a physical device as if connected to the EIS
    implementation's host computer. A relative physical device generates input
    events in mm, an absolute physical device generates input events in mm
    within the device's specified physical size. Physical devices do not have
    regions and no `ei_device.region` events are sent for such devices.
     */
    #[derive(Clone, Copy, Debug)]
    pub enum DeviceType {
        /** a virtual device */
        Virtual = 1,
        /** representation of a physical device */
        Physical = 2,
    }

    impl From<DeviceType> for u32 {
        fn from(value: DeviceType) -> u32 {
            value as u32
        }
    }

    impl crate::OwnedArg for DeviceType {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            todo!()
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        This device has been removed and a client should release all
        associated resources.

        This `ei_device` object will be destroyed by the EIS implementation immmediately after
        after this event is sent and as such the client must not attempt to use
        it after that point.
         */
        Destroyed {
            /** this event's serial number */
            serial: u32,
        },
        /**
        The name of this device, if any. This event is optional and sent once immediately
        after object creation.

        It is a protocol violation to send this event after the `ei_device.done` event.
         */
        Name {
            /** the device name */
            name: String,
        },
        /**
        The device type, one of virtual or physical.

        Devices of type `ei_device.device_type.physical` are supported only clients of
        type `ei_handshake.context_type.receiver.`

        This event is sent once immediately after object creation.
        It is a protocol violation to send this event after the `ei_device.done` event.
         */
        DeviceType {
            /** the device type */
            device_type: DeviceType,
        },
        /**
        The device dimensions in mm. This event is optional and sent once immediately
        after object creation.

        This event is only sent for devices of `ei_device.device_type.physical.`

        It is a protocol violation to send this event after the `ei_device.done` event.
         */
        Dimensions {
            /** the device physical width in mm */
            width: u32,
            /** the device physical height in mm */
            height: u32,
        },
        /**
        Notifies the client of one region. The number of regions is constant for a device
        and all regions are announced immediately after object creation.

        A region is rectangular and defined by an x/y offset and a width and a height.
        A region defines the area on an EIS desktop layout that is accessible by
        this device - this region may not be the full area of the desktop.
        Input events may only be sent for points within the regions.

        The use of regions is private to the EIS compositor and coordinates may not
        match the size of the actual desktop. For example, a compositor may set a
        1920x1080 region to represent a 4K monitor and transparently map input
        events into the respective true pixels.

        Absolute devices may have different regions, it is up to the libei client
        to send events through the correct device to target the right pixel. For
        example, a dual-head setup my have two absolute devices, the first with a
        zero offset region spanning the left screen, the second with a nonzero
        offset spanning the right screen.

        The physical scale denotes a constant factor that needs to be applied to
        any relative movement on this region for that movement to match the same
        *physical* movement on another region.

        It is an EIS implementation bug to advertise the absolute pointer capability
        on a device_type.virtual device without advertising an `ei_region` for this device.

        This event is optional and sent immediately after object creation. Where a device
        has multiple regions, this event is sent once for each region.
        It is a protocol violation to send this event after the `ei_device.done` event.
         */
        Region {
            /** region x offset in logical pixels */
            offset_x: u32,
            /** region y offset in logical pixels */
            offset_y: u32,
            /** region width in logical pixels */
            width: u32,
            /** region height in logical pixels */
            hight: u32,
            /** the physical scale for this region */
            scale: f32,
        },
        /**
        Notification that a new device has a sub-interface.

        This event may be sent for the
        - "`ei_pointer`" interface if the device has the
          `ei_device.capabilities.pointer` capability
        - "`ei_pointer_absolute`" interface if the device has the
          `ei_device.capabilities.pointer_absolute` capability
        - "`ei_scroll`" interface if the device has the
          `ei_device.capabilities.scroll` capability
        - "`ei_button`" interface if the device has the
          `ei_device.capabilities.button` capability
        - "`ei_keyboard`" interface if the device has the
          `ei_device.capabilities.keyboard` capability
        - "`ei_touchscreen`" interface if the device has the
          `ei_device.capabilities.touchscreen` capability
        The interface version is equal or less to the client-supported
        version in `ei_handshake.interface_version` for the respective interface.

        This event is optional and sent immediately after object creation
        and at most once per interface.
        It is a protocol violation to send this event after the `ei_device.done` event.
         */
        Interface {
            /**  */
            object: u64,
            /** the interface name */
            interface_name: String,
            /** the interface version */
            version: u32,
        },
        /**
        Notification that the initial burst of events is complete and
        the client can set up this device now.

        It is a protocol violation to send this event more than once per device.
         */
        Done,
        /**
        Notification that the device has been resumed by the EIS implementation
        and (depending on the `ei_handshake.context_type`) the client may request
        `ei_device.start_emulating` or the EIS implementation may
        `ei_device.start_emulating` events.

        It is a client bug to request emulation of events on a device that is
        not resumed. The EIS implementation may silently discard such events.

        A newly advertised device is in the `ei_device.paused` state.
         */
        Resumed {
            /** this event's serial number */
            serial: u32,
        },
        /**
        Notification that the device has been paused by the EIS implementation
        and no futher events will be accepted on this device until
        it is resumed again.

        For devices of `ei_device_setup.context_type` sender, the client thus does
        not need to request `ei_device.stop_emulating` and may request
        `ei_device.start_emulating` after a subsequent `ei_device.resumed.`

        For devices of `ei_device_setup.context_type` receiver and where
        the EIS implementation did not send a `ei_device.stop_emulating`
        prior to this event, the device may send a `ei_device.start_emulating`
        event after a subsequent `ei_device.resumed` event.

        Pausing a device resets the logical state of the device to neutral.
        This includes:
        - any buttons or keys logically down are released
        - any modifiers logically down are released
        - any touches logically down are released

        It is a client bug to request emulation of events on a device that is
        not resumed. The EIS implementation may silently discard such events.

        A newly advertised device is in the `ei_device.paused` state.
         */
        Paused {
            /** this event's serial number */
            serial: u32,
        },
        /**
        See the `ei_device.start_emulating` request for details.

        It is a protocol violation to send this event for a client
        of an `ei_handshake.context_type` other than receiver.
         */
        StartEmulating {
            /** this event's serial number */
            serial: u32,
            /**  */
            sequence: u32,
        },
        /**
        See the `ei_device.stop_emulating` request for details.

        It is a protocol violation to send this event for a client
        of an `ei_handshake.context_type` other than receiver.
         */
        StopEmulating {
            /** this event's serial number */
            serial: u32,
        },
        /**
        See the `ei_device.frame` request for details.

        It is a protocol violation to send this event for a client
        of an `ei_handshake.context_type` other than receiver.
         */
        Frame {
            /** this event's serial number */
            serial: u32,
            /** timestamp in microseconds */
            timestamp: u64,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::Destroyed {
                    serial: bytes.read_arg()?,
                }),
                1 => Some(Self::Name {
                    name: bytes.read_arg()?,
                }),
                2 => Some(Self::DeviceType {
                    device_type: bytes.read_arg()?,
                }),
                3 => Some(Self::Dimensions {
                    width: bytes.read_arg()?,
                    height: bytes.read_arg()?,
                }),
                4 => Some(Self::Region {
                    offset_x: bytes.read_arg()?,
                    offset_y: bytes.read_arg()?,
                    width: bytes.read_arg()?,
                    hight: bytes.read_arg()?,
                    scale: bytes.read_arg()?,
                }),
                5 => Some(Self::Interface {
                    object: bytes.read_arg()?,
                    interface_name: bytes.read_arg()?,
                    version: bytes.read_arg()?,
                }),
                6 => Some(Self::Done),
                7 => Some(Self::Resumed {
                    serial: bytes.read_arg()?,
                }),
                8 => Some(Self::Paused {
                    serial: bytes.read_arg()?,
                }),
                9 => Some(Self::StartEmulating {
                    serial: bytes.read_arg()?,
                    sequence: bytes.read_arg()?,
                }),
                10 => Some(Self::StopEmulating {
                    serial: bytes.read_arg()?,
                }),
                11 => Some(Self::Frame {
                    serial: bytes.read_arg()?,
                    timestamp: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

/**
Interface for pointer motion requests and events. This interface
is available on devices with the `ei_device.capability` pointer.

This interface is only provided once per device and where a client
requests `ei_pointer.release` the interface does not get re-initialized. An
EIS implementation may adjust the behavior of the device (including removing
the device) if the interface is releasd.

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod pointer {
    #[derive(Clone, Debug)]
    pub struct Pointer {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Pointer {
        const NAME: &'static str = "ei_pointer";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Pointer {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Pointer {
        /**
        Notification that the client is no longer interested in this pointer.
        The EIS implementation will release any resources related to this pointer and
        send the `ei_pointer.destroyed` event once complete.
         */
        pub fn release(&self) -> rustix::io::Result<()> {
            let args = &[];

            self.connection.request(self.id, 0, args)?;

            Ok(())
        }

        /**
        Generate a relative motion event on this pointer.

        It is a client bug to send this request more than once
        within the same `ei_device.frame.`

        It is a client bug to send this request on a device without
        the `ei_device.capabilities.pointer` capability.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than sender.
         */
        pub fn motion_relative(&self, x: f32, y: f32) -> rustix::io::Result<()> {
            let args = &[crate::Arg::Float(x.into()), crate::Arg::Float(y.into())];

            self.connection.request(self.id, 1, args)?;

            Ok(())
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        This object has been removed and a client should release all
        associated resources.

        This object will be destroyed by the EIS implementation immmediately after
        after this event is sent and as such the client must not attempt to use
        it after that point.
         */
        Destroyed {
            /** this event's serial number */
            serial: u32,
        },
        /**
        See the `ei_pointer.motion_relative` request for details.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than receiver.
         */
        MotionRelative {
            /**  */
            x: f32,
            /**  */
            y: f32,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::Destroyed {
                    serial: bytes.read_arg()?,
                }),
                1 => Some(Self::MotionRelative {
                    x: bytes.read_arg()?,
                    y: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

/**
Interface for absolute pointer requests and events. This interface
is available on devices with the `ei_device.capability` pointer_absolute.

This interface is only provided once per device and where a client
requests `ei_pointer_absolute.release` the interface does not get
re-initialized. An EIS implementation may adjust the behavior of the
device (including removing the device) if the interface is releasd.

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod pointer_absolute {
    #[derive(Clone, Debug)]
    pub struct PointerAbsolute {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for PointerAbsolute {
        const NAME: &'static str = "ei_pointer_absolute";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for PointerAbsolute {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl PointerAbsolute {
        /**
        Notification that the client is no longer interested in this object.
        The EIS implementation will release any resources related to this object and
        send the `ei_pointer_absolute.destroyed` event once complete.
         */
        pub fn release(&self) -> rustix::io::Result<()> {
            let args = &[];

            self.connection.request(self.id, 0, args)?;

            Ok(())
        }

        /**
        Generate an absolute motion event on this pointer. The x/y
        coordinates must be within the device's regions or the event
        is silently discarded.

        It is a client bug to send this request more than once
        within the same `ei_device.frame.`

        It is a client bug to send this request on a device without
        the `ei_device.capabilities.pointer_absolute` capability.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than sender.
         */
        pub fn motion_absolute(&self, x: f32, y: f32) -> rustix::io::Result<()> {
            let args = &[crate::Arg::Float(x.into()), crate::Arg::Float(y.into())];

            self.connection.request(self.id, 1, args)?;

            Ok(())
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        This object has been removed and a client should release all
        associated resources.

        This object will be destroyed by the EIS implementation immmediately after
        after this event is sent and as such the client must not attempt to use
        it after that point.
         */
        Destroyed {
            /** this event's serial number */
            serial: u32,
        },
        /**
        See the `ei_pointer_absolute.motion_absolute` request for details.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than receiver.
         */
        MotionAbsolute {
            /**  */
            x: f32,
            /**  */
            y: f32,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::Destroyed {
                    serial: bytes.read_arg()?,
                }),
                1 => Some(Self::MotionAbsolute {
                    x: bytes.read_arg()?,
                    y: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

/**
Interface for scroll requests and events. This interface
is available on devices with the `ei_device.capability` scroll.

This interface is only provided once per device and where a client
requests `ei_scroll.release` the interface does not get
re-initialized. An EIS implementation may adjust the behavior of the
device (including removing the device) if the interface is releasd.

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod scroll {
    #[derive(Clone, Debug)]
    pub struct Scroll {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Scroll {
        const NAME: &'static str = "ei_scroll";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Scroll {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Scroll {
        /**
        Notification that the client is no longer interested in this object.
        The EIS implementation will release any resources related to this object and
        send the `ei_scroll.destroyed` event once complete.
         */
        pub fn release(&self) -> rustix::io::Result<()> {
            let args = &[];

            self.connection.request(self.id, 0, args)?;

            Ok(())
        }

        /**
        Generate a a smooth (pixel-precise) scroll event on this pointer.
        Clients must not send `ei_scroll.scroll_discrete` events for the same event,
        the EIS implementation is responsible for emulation of discrete
        scroll events.

        It is a client bug to send this request more than once
        within the same `ei_device.frame.`

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than sender.
         */
        pub fn scroll(&self, x: f32, y: f32) -> rustix::io::Result<()> {
            let args = &[crate::Arg::Float(x.into()), crate::Arg::Float(y.into())];

            self.connection.request(self.id, 1, args)?;

            Ok(())
        }

        /**
        Generate a a discrete (e.g. wheel) scroll event on this pointer.
        Clients must not send `ei_scroll.scroll` events for the same event,
        the EIS implementation is responsible for emulation of smooth
        scroll events.

        A discrete scroll event is based logical scroll units (equivalent to one
        mouse wheel click). The value for one scroll unit is 120, a fraction or
        multiple thereof represents a fraction or multiple of a wheel click.

        It is a client bug to send this request more than once
        within the same `ei_device.frame.`

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than sender.
         */
        pub fn scroll_discrete(&self, x: i32, y: i32) -> rustix::io::Result<()> {
            let args = &[crate::Arg::Int32(x.into()), crate::Arg::Int32(y.into())];

            self.connection.request(self.id, 2, args)?;

            Ok(())
        }

        /**
        Generate a a scroll stop or cancel event on this pointer.

        A scroll stop event notifies the EIS implementation that the interaction causing a
        scroll motion previously triggered with `ei_scroll.scroll` or
        `ei_scroll.scroll_discrete` has stopped. For example, if all
        fingers are lifted off a touchpad, two-finger scrolling has logically
        stopped. The EIS implementation may use this information to e.g. start kinetic scrolling
        previously based on the previous finger speed.

        If is_cancel is nonzero, the event represents a cancellation of the
        current interaction. This indicates that the interaction has stopped to the
        point where further (server-emulated) scroll events from this device are wrong.

        It is a client bug to send this request more than once
        within the same `ei_device.frame.`

        It is a client bug to send this request for an axis that
        had a a nonzero value in either `ei_scroll.scroll` or `ei_scroll.scroll_discrete`
        in the current frame.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than sender.
         */
        pub fn scroll_stop(&self, x: u32, y: u32, is_cancel: u32) -> rustix::io::Result<()> {
            let args = &[
                crate::Arg::Uint32(x.into()),
                crate::Arg::Uint32(y.into()),
                crate::Arg::Uint32(is_cancel.into()),
            ];

            self.connection.request(self.id, 3, args)?;

            Ok(())
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        This object has been removed and a client should release all
        associated resources.

        This object will be destroyed by the EIS implementation immmediately after
        after this event is sent and as such the client must not attempt to use
        it after that point.
         */
        Destroyed {
            /** this event's serial number */
            serial: u32,
        },
        /**
        See the `ei_scroll.scroll` request for details.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than receiver.
         */
        Scroll {
            /**  */
            x: f32,
            /**  */
            y: f32,
        },
        /**
        See the `ei_scroll.scroll_discrete` request for details.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than receiver.
         */
        ScrollDiscrete {
            /**  */
            x: i32,
            /**  */
            y: i32,
        },
        /**

        See the `ei_scroll.scroll_stop` request for details.
        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than receiver.
         */
        ScrollStop {
            /**  */
            x: u32,
            /**  */
            y: u32,
            /**  */
            is_cancel: u32,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::Destroyed {
                    serial: bytes.read_arg()?,
                }),
                1 => Some(Self::Scroll {
                    x: bytes.read_arg()?,
                    y: bytes.read_arg()?,
                }),
                2 => Some(Self::ScrollDiscrete {
                    x: bytes.read_arg()?,
                    y: bytes.read_arg()?,
                }),
                3 => Some(Self::ScrollStop {
                    x: bytes.read_arg()?,
                    y: bytes.read_arg()?,
                    is_cancel: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

/**
Interface for button requests and events. This interface
is available on devices with the `ei_device.capability` button.

This interface is only provided once per device and where a client
requests `ei_button.release` the interface does not get
re-initialized. An EIS implementation may adjust the behavior of the
device (including removing the device) if the interface is releasd.

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod button {
    #[derive(Clone, Debug)]
    pub struct Button {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Button {
        const NAME: &'static str = "ei_button";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Button {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Button {
        /**
        Notification that the client is no longer interested in this object.
        The EIS implementation will release any resources related to this object and
        send the `ei_button.destroyed` event once complete.
         */
        pub fn release(&self) -> rustix::io::Result<()> {
            let args = &[];

            self.connection.request(self.id, 0, args)?;

            Ok(())
        }

        /**
        Generate a button event on this pointer.

        The button codes must match the defines in linux/input-event-codes.h.

        It is a client bug to send more than one button request for the same button
        within the same `ei_device.frame.`

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than sender.
         */
        pub fn button(&self, button: u32, state: ButtonState) -> rustix::io::Result<()> {
            let args = &[
                crate::Arg::Uint32(button.into()),
                crate::Arg::Uint32(state.into()),
            ];

            self.connection.request(self.id, 1, args)?;

            Ok(())
        }
    }

    /**
    The logical state of a button.
     */
    #[derive(Clone, Copy, Debug)]
    pub enum ButtonState {
        /** the button is logically up */
        Released = 0,
        /** the button is logically down */
        Press = 1,
    }

    impl From<ButtonState> for u32 {
        fn from(value: ButtonState) -> u32 {
            value as u32
        }
    }

    impl crate::OwnedArg for ButtonState {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            todo!()
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        This pointer has been removed and a client should release all
        associated resources.

        This `ei_scroll` object will be destroyed by the EIS implementation immmediately after
        after this event is sent and as such the client must not attempt to use
        it after that point.
         */
        Destroyed {
            /** this event's serial number */
            serial: u32,
        },
        /**
        See the `ei_scroll.button` request for details.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than receiver.

        It is an EIS implementation bug to send more than one button request
        for the same button within the same `ei_device.frame.`
         */
        Button {
            /**  */
            button: u32,
            /**  */
            state: ButtonState,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::Destroyed {
                    serial: bytes.read_arg()?,
                }),
                1 => Some(Self::Button {
                    button: bytes.read_arg()?,
                    state: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

/**
Interface for keyboard requests and events. This interface
is available on devices with the `ei_device.capability` keyboard.

This interface is only provided once per device and where a client
requests `ei_keyboard.release` the interface does not get re-initialized. An
EIS implementation may adjust the behavior of the device (including removing
the device) if the interface is releasd.

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod keyboard {
    #[derive(Clone, Debug)]
    pub struct Keyboard {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Keyboard {
        const NAME: &'static str = "ei_keyboard";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Keyboard {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Keyboard {
        /**
        Notification that the client is no longer interested in this keyboard.
        The EIS implementation will release any resources related to this keyboard and
        send the `ei_keyboard.destroyed` event once complete.
         */
        pub fn release(&self) -> rustix::io::Result<()> {
            let args = &[];

            self.connection.request(self.id, 0, args)?;

            Ok(())
        }

        /**
        Generate a key event on this keyboard. If the device has an
        `ei_device.keymap`, the key code corresponds to that keymap.

        The key codes must match the defines in linux/input-event-codes.h.

        It is a client bug to send more than one key request for the same key
        within the same `ei_device.frame.`

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than sender.
         */
        pub fn key(&self, key: u32, state: KeyState) -> rustix::io::Result<()> {
            let args = &[
                crate::Arg::Uint32(key.into()),
                crate::Arg::Uint32(state.into()),
            ];

            self.connection.request(self.id, 1, args)?;

            Ok(())
        }
    }

    /**
    The logical state of a key.
     */
    #[derive(Clone, Copy, Debug)]
    pub enum KeyState {
        /** the key is logically up */
        Released = 0,
        /** the key is logically down */
        Press = 1,
    }

    impl From<KeyState> for u32 {
        fn from(value: KeyState) -> u32 {
            value as u32
        }
    }

    impl crate::OwnedArg for KeyState {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            todo!()
        }
    }
    /**
    The keymap type describes how the keymap in the `ei_keyboard.keymap` event
    should be parsed.
     */
    #[derive(Clone, Copy, Debug)]
    pub enum KeymapType {
        /** a libxkbcommon-compatible XKB keymap */
        Xkb = 1,
    }

    impl From<KeymapType> for u32 {
        fn from(value: KeymapType) -> u32 {
            value as u32
        }
    }

    impl crate::OwnedArg for KeymapType {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            todo!()
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        This keyboard has been removed and a client should release all
        associated resources.

        This `ei_keyboard` object will be destroyed by the EIS implementation immmediately after
        after this event is sent and as such the client must not attempt to use
        it after that point.
         */
        Destroyed {
            /** this event's serial number */
            serial: u32,
        },
        /**
        Notification that this device has a keymap. Future key events must be
        interpreted by the client according to this keymap. For clients
        of `ei_handshake.context_type` sender it is the client's
        responsibility to send the correct `ei_keyboard.key` keycodes to
        generate the expected keysym in the EIS implementation.

        The keymap is constant for the lifetime of the device.

        This event provides a file descriptor to the client which can be
        memory-mapped in read-only mode to provide a keyboard mapping
        description. The fd must be mapped with MAP_PRIVATE by
        the recipient, as MAP_SHARED may fail.

        This event is sent immediately after the `ei_keyboard` object is created
        and before the `ei_device.done` event. It is a protocol violation to send this
        event after the `ei_device.done` event.
         */
        Keymap {
            /** the keymap type */
            keymap_type: KeymapType,
            /** the keymap size in bytes */
            size: u32,
            /** file descriptor to the keymap */
            keymap: std::os::unix::io::OwnedFd,
        },
        /**
        See the `ei_keyboard.key` request for details.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than receiver.

        It is a protocol violation to send a key down event in the same
        frame as a key up event for the same key in the same frame.
         */
        Key {
            /**  */
            key: u32,
            /**  */
            state: KeyState,
        },
        /**
        Notification that the EIS implementation has changed modifier
        states on this device. Future `ei_keyboard.key` requests must take the
        new modifier state into account.

        A client must assume that all modifiers are lifted when it
        receives an `ei_device.paused` event. The EIS implementation
        must send this event after `ei_device.resumed` to notify the client
        of any nonzero modifier state.

        This event does not reqire an `ei_device.frame` and should
        be processed immediately by the client.

        This event is only sent for devices with an `ei_device.keymap.`
         */
        Modifiers {
            /** this event's serial number */
            serial: u32,
            /** depressed modifiers */
            depressed: u32,
            /** locked modifiers */
            locked: u32,
            /** latched modifiers */
            latched: u32,
            /** the keyboard group (layout) */
            group: u32,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::Destroyed {
                    serial: bytes.read_arg()?,
                }),
                1 => Some(Self::Keymap {
                    keymap_type: bytes.read_arg()?,
                    size: bytes.read_arg()?,
                    keymap: bytes.read_arg()?,
                }),
                2 => Some(Self::Key {
                    key: bytes.read_arg()?,
                    state: bytes.read_arg()?,
                }),
                3 => Some(Self::Modifiers {
                    serial: bytes.read_arg()?,
                    depressed: bytes.read_arg()?,
                    locked: bytes.read_arg()?,
                    latched: bytes.read_arg()?,
                    group: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

/**
Interface for touchscreen requests and events. This interface
is available on devices with the `ei_device.capability` touchscreen.

This interface is only provided once per device and where a client
requests `ei_touchscreen.release` the interface does not get re-initialized. An
EIS implementation may adjust the behavior of the device (including removing
the device) if the interface is releasd.

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod touchscreen {
    #[derive(Clone, Debug)]
    pub struct Touchscreen {
        pub(crate) connection: crate::Connection,
        pub(crate) id: u64,
    }

    impl crate::Interface for Touchscreen {
        const NAME: &'static str = "ei_touchscreen";
        const VERSION: u32 = 1;
        type Incoming = Event;
    }

    impl crate::OwnedArg for Touchscreen {
        fn parse(buf: &mut crate::ByteStream) -> Option<Self> {
            let id = u64::parse(buf)?;
            Some(Self {
                connection: buf.connection().clone(),
                id,
            })
        }
    }

    impl Touchscreen {
        /**
        Notification that the client is no longer interested in this touch.
        The EIS implementation will release any resources related to this touch and
        send the `ei_touch.destroyed` event once complete.
         */
        pub fn release(&self) -> rustix::io::Result<()> {
            let args = &[];

            self.connection.request(self.id, 0, args)?;

            Ok(())
        }

        /**
        Notifies the EIS implementation about a new touch logically down at the
        given coordinates. The touchid is a unique id for this touch. Touchids
        may be re-used after `ei_touchscreen.up.`

        The x/y coordinates must be within the device's regions or the event and future
        `ei_touchscreen.motion` events with the same touchid are silently discarded.

        It is a protocol violation to send a touch down in the same
        frame as a touch motion or touch up.
         */
        pub fn down(&self, touchid: u32, x: f32, y: f32) -> rustix::io::Result<()> {
            let args = &[
                crate::Arg::Uint32(touchid.into()),
                crate::Arg::Float(x.into()),
                crate::Arg::Float(y.into()),
            ];

            self.connection.request(self.id, 1, args)?;

            Ok(())
        }

        /**
        Notifies the EIS implementation about an existing touch changing position to
        the given coordinates. The touchid is the unique id for this touch previously
        sent with `ei_touchscreen.down.`

        The x/y coordinates must be within the device's regions or the event is
        silently discarded.

        It is a protocol violation to send a touch motion in the same
        frame as a touch down or touch up.
         */
        pub fn motion(&self, touchid: u32, x: f32, y: f32) -> rustix::io::Result<()> {
            let args = &[
                crate::Arg::Uint32(touchid.into()),
                crate::Arg::Float(x.into()),
                crate::Arg::Float(y.into()),
            ];

            self.connection.request(self.id, 2, args)?;

            Ok(())
        }

        /**
        Notifies the EIS implementation about an existing touch being logically
        up. The touchid is the unique id for this touch previously
        sent with `ei_touchscreen.down.`

        The touchid may be re-used after this request.

        It is a protocol violation to send a touch up in the same
        frame as a touch motion or touch down.
         */
        pub fn up(&self, touchid: u32) -> rustix::io::Result<()> {
            let args = &[crate::Arg::Uint32(touchid.into())];

            self.connection.request(self.id, 3, args)?;

            Ok(())
        }
    }

    #[non_exhaustive]
    #[derive(Debug)]
    pub enum Event {
        /**
        This touch has been removed and a client should release all
        associated resources.

        This `ei_touchscreen` object will be destroyed by the EIS implementation immmediately after
        after this event is sent and as such the client must not attempt to use
        it after that point.
         */
        Destroyed {
            /** this event's serial number */
            serial: u32,
        },
        /**
        See the `ei_touchscreen.down` request for details.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than receiver.

        It is a protocol violation to send a touch down in the same
        frame as a touch motion or touch up.
         */
        Down {
            /**  */
            touchid: u32,
            /**  */
            x: f32,
            /**  */
            y: f32,
        },
        /**
        See the `ei_touchscreen.motion` request for details.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than receiver.

        It is a protocol violation to send a touch motion in the same
        frame as a touch down or touch up.
         */
        Motion {
            /**  */
            touchid: u32,
            /**  */
            x: f32,
            /**  */
            y: f32,
        },
        /**
        See the `ei_touchscreen.up` request for details.

        It is a protocol violation to send this request for a client
        of an `ei_handshake.context_type` other than receiver.

        It is a protocol violation to send a touch up in the same
        frame as a touch motion or touch down.
         */
        Up {
            /**  */
            touchid: u32,
        },
    }

    impl Event {
        pub(super) fn parse(operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
            match operand {
                0 => Some(Self::Destroyed {
                    serial: bytes.read_arg()?,
                }),
                1 => Some(Self::Down {
                    touchid: bytes.read_arg()?,
                    x: bytes.read_arg()?,
                    y: bytes.read_arg()?,
                }),
                2 => Some(Self::Motion {
                    touchid: bytes.read_arg()?,
                    x: bytes.read_arg()?,
                    y: bytes.read_arg()?,
                }),
                3 => Some(Self::Up {
                    touchid: bytes.read_arg()?,
                }),
                _ => None,
            }
        }
    }
}

// map opcodes

#[non_exhaustive]
#[derive(Debug)]
pub enum Event {
    Handshake(handshake::Event),
    Connection(connection::Event),
    Callback(callback::Event),
    Pingpong(pingpong::Event),
    Seat(seat::Event),
    Device(device::Event),
    Pointer(pointer::Event),
    PointerAbsolute(pointer_absolute::Event),
    Scroll(scroll::Event),
    Button(button::Event),
    Keyboard(keyboard::Event),
    Touchscreen(touchscreen::Event),
}

impl Event {
    fn parse(interface: &'static str, operand: u32, bytes: &mut crate::ByteStream) -> Option<Self> {
        match interface {
            "ei_handshake" => Some(Self::Handshake(handshake::Event::parse(operand, bytes)?)),
            _ => None,
            "ei_connection" => Some(Self::Connection(connection::Event::parse(operand, bytes)?)),
            _ => None,
            "ei_callback" => Some(Self::Callback(callback::Event::parse(operand, bytes)?)),
            _ => None,
            "ei_pingpong" => Some(Self::Pingpong(pingpong::Event::parse(operand, bytes)?)),
            _ => None,
            "ei_seat" => Some(Self::Seat(seat::Event::parse(operand, bytes)?)),
            _ => None,
            "ei_device" => Some(Self::Device(device::Event::parse(operand, bytes)?)),
            _ => None,
            "ei_pointer" => Some(Self::Pointer(pointer::Event::parse(operand, bytes)?)),
            _ => None,
            "ei_pointer_absolute" => Some(Self::PointerAbsolute(pointer_absolute::Event::parse(
                operand, bytes,
            )?)),
            _ => None,
            "ei_scroll" => Some(Self::Scroll(scroll::Event::parse(operand, bytes)?)),
            _ => None,
            "ei_button" => Some(Self::Button(button::Event::parse(operand, bytes)?)),
            _ => None,
            "ei_keyboard" => Some(Self::Keyboard(keyboard::Event::parse(operand, bytes)?)),
            _ => None,
            "ei_touchscreen" => Some(Self::Touchscreen(touchscreen::Event::parse(
                operand, bytes,
            )?)),
            _ => None,
        }
    }
}
