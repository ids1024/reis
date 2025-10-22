#![allow(
    unused_imports,
    unused_parens,
    clippy::useless_conversion,
    clippy::double_parens,
    clippy::match_single_binding,
    clippy::unused_unit
)]

// GENERATED FILE

pub(crate) mod handshake {
    use crate::wire;

    /// Context types for connections.
    ///
    /**
    Context types for connections. The context type for a connection is set
    once in the `ei_handshake.context_type` request.
     */
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
    pub enum ContextType {
        /// The ei client receives input events from the eis implementation.
        Receiver = 1,
        /// The ei client sends input events to the eis implementation.
        Sender = 2,
    }

    impl From<ContextType> for u32 {
        fn from(value: ContextType) -> u32 {
            value as u32
        }
    }

    impl wire::OwnedArg for ContextType {
        fn parse(buf: &mut wire::ByteStream) -> Result<Self, wire::ParseError> {
            match u32::parse(buf)? {
                1 => Ok(Self::Receiver),
                2 => Ok(Self::Sender),
                variant => Err(wire::ParseError::InvalidVariant("ContextType", variant)),
            }
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            wire::Arg::Uint32(*self as u32)
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

pub(crate) mod connection {
    use crate::wire;

    /// Disconnection reason.
    ///
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
        /// Client was purposely disconnected.
        Disconnected = 0,
        /// An error caused the disconnection.
        Error = 1,
        /// Sender/receiver client sent request for receiver/sender mode.
        Mode = 2,
        /// Client committed a protocol violation.
        Protocol = 3,
        /// Client sent an invalid value.
        Value = 4,
        /// Error on the transport layer.
        Transport = 5,
    }

    impl From<DisconnectReason> for u32 {
        fn from(value: DisconnectReason) -> u32 {
            value as u32
        }
    }

    impl wire::OwnedArg for DisconnectReason {
        fn parse(buf: &mut wire::ByteStream) -> Result<Self, wire::ParseError> {
            match u32::parse(buf)? {
                0 => Ok(Self::Disconnected),
                1 => Ok(Self::Error),
                2 => Ok(Self::Mode),
                3 => Ok(Self::Protocol),
                4 => Ok(Self::Value),
                5 => Ok(Self::Transport),
                variant => Err(wire::ParseError::InvalidVariant(
                    "DisconnectReason",
                    variant,
                )),
            }
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            wire::Arg::Uint32(*self as u32)
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

pub(crate) mod callback {
    use crate::wire;
}

pub(crate) mod pingpong {
    use crate::wire;
}

pub(crate) mod seat {
    use crate::wire;
}

pub(crate) mod device {
    use crate::wire;

    /// Device type.
    ///
    /**
    If the device type is `ei_device.device_type.virtual`, the device is a
    virtual device representing input as applied on the EIS implementation's
    screen. A relative virtual device generates input events in logical pixels,
    an absolute virtual device generates input events in logical pixels on one
    of the device's regions. Virtual devices do not have a `ei_device.dimension` but
    it may have an `ei_device.region`.

    If the device type is `ei_device.device_type.physical`, the device is a
    representation of a physical device as if connected to the EIS
    implementation's host computer. A relative physical device generates input
    events in mm, an absolute physical device generates input events in mm
    within the device's specified physical size. Physical devices do not have
    regions and no `ei_device.region` events are sent for such devices.
     */
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
    pub enum DeviceType {
        /// A virtual device.
        Virtual = 1,
        /// Representation of a physical device.
        Physical = 2,
    }

    impl From<DeviceType> for u32 {
        fn from(value: DeviceType) -> u32 {
            value as u32
        }
    }

    impl wire::OwnedArg for DeviceType {
        fn parse(buf: &mut wire::ByteStream) -> Result<Self, wire::ParseError> {
            match u32::parse(buf)? {
                1 => Ok(Self::Virtual),
                2 => Ok(Self::Physical),
                variant => Err(wire::ParseError::InvalidVariant("DeviceType", variant)),
            }
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            wire::Arg::Uint32(*self as u32)
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

pub(crate) mod pointer {
    use crate::wire;
}

pub(crate) mod pointer_absolute {
    use crate::wire;
}

pub(crate) mod scroll {
    use crate::wire;
}

pub(crate) mod button {
    use crate::wire;

    /// Button state.
    ///
    /**
    The logical state of a button.
     */
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
    pub enum ButtonState {
        /// The button is logically up.
        Released = 0,
        /// The button is logically down.
        Press = 1,
    }

    impl From<ButtonState> for u32 {
        fn from(value: ButtonState) -> u32 {
            value as u32
        }
    }

    impl wire::OwnedArg for ButtonState {
        fn parse(buf: &mut wire::ByteStream) -> Result<Self, wire::ParseError> {
            match u32::parse(buf)? {
                0 => Ok(Self::Released),
                1 => Ok(Self::Press),
                variant => Err(wire::ParseError::InvalidVariant("ButtonState", variant)),
            }
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            wire::Arg::Uint32(*self as u32)
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

pub(crate) mod keyboard {
    use crate::wire;

    /// Key state.
    ///
    /**
    The logical state of a key.
     */
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
    pub enum KeyState {
        /// The key is logically up.
        Released = 0,
        /// The key is logically down.
        Press = 1,
    }

    impl From<KeyState> for u32 {
        fn from(value: KeyState) -> u32 {
            value as u32
        }
    }

    impl wire::OwnedArg for KeyState {
        fn parse(buf: &mut wire::ByteStream) -> Result<Self, wire::ParseError> {
            match u32::parse(buf)? {
                0 => Ok(Self::Released),
                1 => Ok(Self::Press),
                variant => Err(wire::ParseError::InvalidVariant("KeyState", variant)),
            }
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            wire::Arg::Uint32(*self as u32)
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
    /// The keymap type.
    ///
    /**
    The keymap type describes how the keymap in the `ei_keyboard.keymap` event
    should be parsed.
     */
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
    pub enum KeymapType {
        /// A libxkbcommon-compatible xkb keymap.
        Xkb = 1,
    }

    impl From<KeymapType> for u32 {
        fn from(value: KeymapType) -> u32 {
            value as u32
        }
    }

    impl wire::OwnedArg for KeymapType {
        fn parse(buf: &mut wire::ByteStream) -> Result<Self, wire::ParseError> {
            match u32::parse(buf)? {
                1 => Ok(Self::Xkb),
                variant => Err(wire::ParseError::InvalidVariant("KeymapType", variant)),
            }
        }

        fn as_arg(&self) -> wire::Arg<'_> {
            wire::Arg::Uint32(*self as u32)
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

pub(crate) mod touchscreen {
    use crate::wire;
}
