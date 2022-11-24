extern crate snap_rs_proc_macros;

pub mod serial;
pub mod types;

pub(self) use snap_rs_proc_macros::packets;

use types::*;

/*
macro_rules! packet {
    {@ $decoder:ident $param:ident $type:ty, none} => {
        let $param = <$type as serial::Decode>::decode($decoder)?;
    };
    {@ $decoder:ident $param:ident Vec<$type:ty>, remain} => {
        let mut $param = Vec::new();
        while $decoder.remaining() > 0 {
            $param.push(<$type as serial::Decode>::decode($decoder)?);
        }
    };
    {@ $decoder:ident $param:ident Vec<$type:ty>, $length:ident} => {
        // Can express $length as a u32 as the protocol only ever uses v32 for Vec lengths
        let mut $param = Vec::with_capacity(u32::from($length) as usize);
        for _ in 0..u32::from($length) {
            $param.push(<$type as serial::Decode>::decode($decoder)?);
        }
    };

    // Encoder
    {@ $encoder:ident $self:ident $param:ident none} => {
        serial::Encode::encode(&$self.$param, $encoder)?;
    };
    {@ $encoder:ident $self:ident $param:ident remain} => {
        for b in &$self.$param {
            serial::Encode::encode(b, $encoder)?;
        }
    };
    {@ $encoder:ident $self:ident $param:ident $length:ident} => {
        // TODO: do this automagically
        assert_eq!($self.$param.len(), u32::from($self.$length) as usize);
        for b in &$self.$param {
            serial::Encode::encode(b, $encoder)?;
        }
    };

    // Expand to Struct
    {@ $name:ident { } -> ($(pub $param:ident : $type:tt $(<$inner:tt>)? => $length:ident),* $(,)?)} => {
        pub struct $name {
            $(pub $param : $type $(<$inner>)?),*
        }
        impl serial::Decode for $name {
            fn decode(_decoder: &mut serial::Decoder) -> Result<Self, serial::DecodeError> {
                $(packet!{@ _decoder $param $type $(<$inner>)?, $length})*

                Ok(Self {
                    $($param),*
                })
            }
        }
        impl serial::Encode for $name {
            fn encode(&self, _encoder: &mut serial::Encoder) -> Result<(), serial::EncodeError> {
                $(packet!{@ _encoder self $param $length})*

                Ok(())
            }
        }
    };
    {@ $name:ident : Ignore { } -> ($(pub $param:ident : $type:tt $(<$inner:tt>)? => $length:ident),* $(,)?)} => {
        pub struct $name {
            $(pub $param : $type $(<$inner>)?),*
        }
    };

    // Parse Struct
    {@ $name:ident $(: $extra:ident)? { $param:ident : Vec<$type:tt, remain>, $($rest:tt)* } -> ($($result:tt)*)} => {
        packet! {
            @ $name $(: $extra)? {
                $($rest)*
            } -> (
                $($result)*
                pub $param : Vec<$type> => remain,
            )
        }
    };
    {@ $name:ident $(: $extra:ident)? { $param:ident : Vec<$type:tt, $length:ident>, $($rest:tt)* } -> ($($result:tt)*)} => {
        packet! {
            @ $name $(: $extra)? {
                $($rest)*
            } -> (
                $($result)*
                pub $param : Vec<$type> => $length,
            )
        }
    };
    {@ $name:ident $(: $extra:ident)? { $param:ident : $type:tt $(<$inner:tt>)?, $($rest:tt)* } -> ($($result:tt)*)} => {
        packet! {
            @ $name $(: $extra)? {
                $($rest)*
            } -> (
                $($result)*
                pub $param : $type $(<$inner>)? => none,
            )
        }
    };

    // Entrypoint
    ($name:ident $(: $extra:ident)? { $( $param:ident : $type:tt $(<$inner:tt $(, $length:ident)?>)?, )* $(,)* }) => {
        packet! {
            @ $name $(: $extra)? { $($param : $type $(<$inner $(, $length)?>)?,)* } -> ()
        }
    };
}

macro_rules! ignore {
    ($extra:block, $extra2:block, Ignore) => {
        $extra2
    };
    ($extra:block, $extra2:block) => {
        $extra
    };
}

macro_rules! packets {
    { $($dir:ident => { $($state:ident => { $($id:expr => $name:ident $(: $extra:ident)? { $($inner:tt)* } $(,)?)* }),* }),* } => {
        paste::paste! {
            #[allow(dead_code)]
            pub enum Packets {
                $($($([<$dir:camel $state:camel $name:camel>](Box<[<$dir:lower>]::[<$state:lower _packets>]::$name>),)*)*)*
            }

            $($($(
                impl From<[<$dir:lower>]::[<$state:lower _packets>]::$name> for Packets {
                    fn from(packet: [<$dir:lower>]::[<$state:lower _packets>]::$name) -> Self {
                        Self::[<$dir:camel $state:camel $name:camel>](Box::new(packet))
                    }
                }
            )*)*)*

            impl Packets {
                pub fn get_id(&self) -> u8 {
                    match self {
                        $($($(Self::[<$dir:camel $state:camel $name:camel>](..) => $id,)*)*)*
                    }
                }

                #[allow(unused_variables)]
                pub fn get_data(&self) -> Vec<u8> {
                    match self {
                        $($($(Self::[<$dir:camel $state:camel $name:camel>](packet) => ignore!({ serial::encode_to_vec(packet.as_ref()).unwrap() }, { Vec::new() } $(, $extra)?),)*)*)*
                    }
                }
            }

            impl std::fmt::Debug for Packets {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "Packets::")?;
                    match self {
                        $($($(Self::[<$dir:camel $state:camel $name:camel>](..) => write!(f, stringify!([<$dir:camel $state:camel $name:camel>])).unwrap(),)*)*)*
                    };
                    Ok(())
                }
            }

            $(pub mod [<$dir:lower>] {
                $(
                    pub use [<$state:lower _packets>]::decode_packet as [<decode_ $state:lower>];
                    pub mod [<$state:lower _packets>] {
                        #[allow(unused_imports)]
                        use crate::packets::types::*;

                        use log::error;

                        use super::super::Packets;

                        #[allow(unused_imports)]
                        use super::super::serial;

                        // This is being used elsewhere. Clientbound decodes aren't being used however, so Clippy complains about those.
                        #[allow(dead_code, unused_variables)]
                        pub fn decode_packet(id: u8, data: Vec<u8>) -> Option<Packets> {
                            match id {
                                $(
                                    $id => {
                                        return ignore!(
                                            {
                                                let (packet, _) = serial::decode_from_slice::<$name>(&data).unwrap();
                                                Some(Packets::[<$dir:camel $state:camel $name:camel>](Box::new(packet)))
                                            },
                                            {
                                                error!("Failed to decode packet with id {}", id);
                                                None
                                            }
                                            $(, $extra)?
                                        );
                                    }
                                )*
                                _ => {
                                    error!("Unknown packet id: {}", id);
                                    None
                                },
                            }
                        }
                        $(
                            packet!( $name $(: $extra)? { $($inner)* } );
                        )*
                    }

                )*
            })*
        }
    }
}

// TODO: Get these in the macro somehow

// Won't actually ever be serialized. Just used for the macro to be happy
#[derive(PartialEq, Eq)]
pub enum PacketState {
    Handshake,
    Status,
    Login,
    Play,
}

impl serial::Decode for PacketState {
    fn decode(decoder: &mut serial::Decoder) -> Result<Self, serial::DecodeError> {
        Ok(<u8 as serial::Decode>::decode(decoder)?.into())
    }
}

impl serial::Encode for PacketState {
    fn encode(&self, encoder: &mut serial::Encoder) -> Result<(), serial::EncodeError> {
        <u8 as serial::Encode>::encode(&u8::from(self), encoder)
    }
}

impl From<u8> for PacketState {
    fn from(id: u8) -> Self {
        match id {
            0 => Self::Handshake,
            1 => Self::Status,
            2 => Self::Login,
            3 => Self::Play,
            _ => panic!("Unknown packet state id: {}", id),
        }
    }
}

impl From<&PacketState> for u8 {
    fn from(state: &PacketState) -> Self {
        match state {
            PacketState::Handshake => 0,
            PacketState::Status => 1,
            PacketState::Login => 2,
            PacketState::Play => 3,
        }
    }
}
*/

/*
    Most of the Packet format can be inferred from the packets below, but here's some of the weirder parts:
    - The length fields in a Vec(Vec<type, length>) will refer to a previous field in the struct. This field will then
      be removed from the struct and will only exist during decoding/encoding.
    - Most of the parameters in a Struct is going to be automatically inferred to a public visability.
      The only situation you would manually specify a `pub` visability is if you want to ensure a length field is kept.
    - Trailing commas can be used anywhere except for the Direction field - The outermost field in the macro.
*/
packets! {
    Serverbound => {
        Handshaking => {
            0x00 => Handshake {
                protocol_version: v32,
                server_address: BoundedString<255>,
                server_port: u16,
                next_state: u8, // is technically a varint, but the valid range is within a u8
            },
        },
        Status => {
            0x00 => Request {},
            0x01 => Ping {
                payload: i64,
            },
        },
        Login => {
            0x00 => LoginStart {
                name: BoundedString<16>,
            },
            0x01 => EncryptionResponse {
                pub shared_secret_length: v32,
                shared_secret: Vec<u8, shared_secret_length>,
                pub verify_token_length: v32,
                verify_token: Vec<u8, verify_token_length>,
            },
        }
    },
    Clientbound => {
        Status => {
            0x00 => Response {
                json_response: BoundedString<32767>,
            },
            0x01 => Pong {
                payload: i64,
            },
        },
        Login => {
            0x00 => Disconnect {
                reason: Chat,
            },
            0x01 => EncryptionRequest {
                server_id: BoundedString<20>, // Appears to be empty/unused
                public_key_length: v32,
                public_key: Vec<u8, public_key_length>,
                verify_token_length: v32,
                verify_token: Vec<u8, verify_token_length>,
            },
            0x02 => LoginSuccess {
                uuid: BoundedString<36>,
                username: BoundedString<16>,
            },
            0x03 => SetCompression {
                threshold: v32,
            },
        },
        Play => {
            0x19 => Disconnect {
                reason: Chat,
            },
        }
    },
    Internal => {
        Server => {
            0x00 => Initalize : Ignore {
                uuid: String,
                username: String,
            },
        },
        Network => {
            0x00 => Disconnect {
                reason: BoundedString<32767>,
            },
        }
    }
}

impl serial::Decode for Packets {
    fn decode(_decoder: &mut serial::Decoder) -> Result<Self, serial::DecodeError> {
        panic!("Decode is not implemented for Packets");
    }
}
