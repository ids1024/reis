#![allow(
    unused_parens,
    clippy::useless_conversion,
    clippy::double_parens,
    clippy::match_single_binding,
    clippy::unused_unit
)]

// GENERATED FILE

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
    /**
    This enum denotes context types for the libei context.

    A context type of receiver is a libei context receiving events
    from the EIS implementation. A context type of sender is a libei context
    sending events to the EIS implementation.
     */
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
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
        fn parse(buf: &mut crate::ByteStream) -> Result<Self, crate::ParseError> {
            match u32::parse(buf)? {
                1 => Ok(Self::Receiver),
                2 => Ok(Self::Sender),
                variant => Err(crate::ParseError::InvalidVariant("ContextType", variant)),
            }
        }

        fn as_arg(&self) -> crate::Arg<'_> {
            crate::Arg::Uint32(*self as u32)
        }

        fn enum_name(&self) -> Option<(&'static str, &'static str)> {
            Some((
                "context_type",
                match self {
                    Self::Receiver => "receiver",
                    Self::Sender => "sender",
                },
            ))
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
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
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
        fn parse(buf: &mut crate::ByteStream) -> Result<Self, crate::ParseError> {
            match u32::parse(buf)? {
                0 => Ok(Self::Disconnected),
                1 => Ok(Self::Error),
                2 => Ok(Self::Mode),
                3 => Ok(Self::Protocol),
                4 => Ok(Self::Value),
                5 => Ok(Self::Transport),
                variant => Err(crate::ParseError::InvalidVariant(
                    "DisconnectReason",
                    variant,
                )),
            }
        }

        fn as_arg(&self) -> crate::Arg<'_> {
            crate::Arg::Uint32(*self as u32)
        }

        fn enum_name(&self) -> Option<(&'static str, &'static str)> {
            Some((
                "disconnect_reason",
                match self {
                    Self::Disconnected => "disconnected",
                    Self::Error => "error",
                    Self::Mode => "mode",
                    Self::Protocol => "protocol",
                    Self::Value => "value",
                    Self::Transport => "transport",
                },
            ))
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
pub mod callback {}

/**
Interface for ensuring a roundtrip to the client implementation.
This interface is identical to `ei_callback` but is intended for
the EIS implementation to enforce a roundtrip to the client.

Note that for a client to receive objects of this type, it must announce
support for this interface in `ei_handshake.interface_version.`
 */
pub mod pingpong {}

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
pub mod seat {}

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
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
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
        fn parse(buf: &mut crate::ByteStream) -> Result<Self, crate::ParseError> {
            match u32::parse(buf)? {
                1 => Ok(Self::Virtual),
                2 => Ok(Self::Physical),
                variant => Err(crate::ParseError::InvalidVariant("DeviceType", variant)),
            }
        }

        fn as_arg(&self) -> crate::Arg<'_> {
            crate::Arg::Uint32(*self as u32)
        }

        fn enum_name(&self) -> Option<(&'static str, &'static str)> {
            Some((
                "device_type",
                match self {
                    Self::Virtual => "virtual",
                    Self::Physical => "physical",
                },
            ))
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
pub mod pointer {}

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
pub mod pointer_absolute {}

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
pub mod scroll {}

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
    /**
    The logical state of a button.
     */
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
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
        fn parse(buf: &mut crate::ByteStream) -> Result<Self, crate::ParseError> {
            match u32::parse(buf)? {
                0 => Ok(Self::Released),
                1 => Ok(Self::Press),
                variant => Err(crate::ParseError::InvalidVariant("ButtonState", variant)),
            }
        }

        fn as_arg(&self) -> crate::Arg<'_> {
            crate::Arg::Uint32(*self as u32)
        }

        fn enum_name(&self) -> Option<(&'static str, &'static str)> {
            Some((
                "button_state",
                match self {
                    Self::Released => "released",
                    Self::Press => "press",
                },
            ))
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
    /**
    The logical state of a key.
     */
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
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
        fn parse(buf: &mut crate::ByteStream) -> Result<Self, crate::ParseError> {
            match u32::parse(buf)? {
                0 => Ok(Self::Released),
                1 => Ok(Self::Press),
                variant => Err(crate::ParseError::InvalidVariant("KeyState", variant)),
            }
        }

        fn as_arg(&self) -> crate::Arg<'_> {
            crate::Arg::Uint32(*self as u32)
        }

        fn enum_name(&self) -> Option<(&'static str, &'static str)> {
            Some((
                "key_state",
                match self {
                    Self::Released => "released",
                    Self::Press => "press",
                },
            ))
        }
    }
    /**
    The keymap type describes how the keymap in the `ei_keyboard.keymap` event
    should be parsed.
     */
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
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
        fn parse(buf: &mut crate::ByteStream) -> Result<Self, crate::ParseError> {
            match u32::parse(buf)? {
                1 => Ok(Self::Xkb),
                variant => Err(crate::ParseError::InvalidVariant("KeymapType", variant)),
            }
        }

        fn as_arg(&self) -> crate::Arg<'_> {
            crate::Arg::Uint32(*self as u32)
        }

        fn enum_name(&self) -> Option<(&'static str, &'static str)> {
            Some((
                "keymap_type",
                match self {
                    Self::Xkb => "xkb",
                },
            ))
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
pub mod touchscreen {}
