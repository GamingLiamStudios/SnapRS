pub mod serial;
pub mod types;

macro_rules! packet {
    {@ $decoder:ident $param:ident $type:ty, none} => {
        let $param = <$type as serial::Decode>::decode($decoder)?;
    };
    {@ $decoder:ident $param:ident $type:ty, remain} => {
        todo!("Implement remain dec");
    };
    {@ $decoder:ident $param:ident $type:ty, $length:ident} => {
        let mut $param = Vec::with_capacity(u32::from($length) as usize);
        for _ in 0..u32::from($length) {
            $param.push(<u8 as serial::Decode>::decode($decoder)?);
        }
    };
    {@ $encoder:ident $self:ident $param:ident none} => {
        serial::Encode::encode(&$self.$param, $encoder)?;
    };
    {@ $encoder:ident $self:ident $param:ident remain} => {
        for b in &$self.$param {
            serial::Encode::encode(b, $encoder)?;
        }
    };
    {@ $encoder:ident $self:ident $param:ident $length:ident} => {
        for b in &$self.$param {
            serial::Encode::encode(b, $encoder)?;
        }
    };
    {@ $name:ident { } -> ($(pub $param:ident : $type:ty => $length:ident),* $(,)?)} => {
        pub struct $name {
            $(pub $param : $type),*
        }
        impl serial::Decode for $name {
            fn decode(_decoder: &mut serial::Decoder) -> Result<Self, serial::DecodeError> {
                $(packet!{@ _decoder $param $type, $length})*

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
    {@ $name:ident { $param:ident : Vec<$type:ty, remain>, $($rest:tt)* } -> ($($result:tt)*)} => {
        packet! {
            @ $name {
                $($rest)*
            } -> (
                $($result)*
                pub $param : Vec<$type> => remain,
            )
        }
    };
    {@ $name:ident { $param:ident : Vec<$type:ty, $length:ident>, $($rest:tt)* } -> ($($result:tt)*)} => {
        packet! {
            @ $name {
                $($rest)*
            } -> (
                $($result)*
                pub $param : Vec<$type> => $length,
            )
        }
    };
    {@ $name:ident { $param:ident : $type:ty, $($rest:tt)* } -> ($($result:tt)*)} => {
        packet! {
            @ $name {
                $($rest)*
            } -> (
                $($result)*
                pub $param : $type => none,
            )
        }
    };
    ($name:ident { $( $param:ident : $type:tt $(<$inner:tt $(, $length:ident)?>)?, )* $(,)* }) => {
        packet! {
            @ $name { $($param : $type $(<$inner $(, $length)?>)?,)* } -> ()
        }
    };
}

macro_rules! packets {
    { $($dir:ident => { $($state:ident => { $($id:expr => $name:ident { $($inner:tt)* } $(,)?)* }),* }),* } => {
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

                pub fn get_data(&self) -> Vec<u8> {
                    match self {
                        $($($(Self::[<$dir:camel $state:camel $name:camel>](packet) => serial::encode_to_vec(packet.as_ref()).unwrap(),)*)*)*
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

                        use super::super::serial;

                        pub fn decode_packet(id: u8, data: Vec<u8>) -> Option<Packets> {
                            match id {
                                $(
                                    $id => {
                                        let (packet, _) = serial::decode_from_slice::<$name>(data.as_slice()).unwrap();
                                        Some(Packets::[<$dir:camel $state:camel $name:camel>](Box::new(packet)))
                                    },
                                )*
                                _ => {
                                    error!("Unknown packet id: {}", id);
                                    None
                                },
                            }
                        }
                        $(
                            packet!( $name { $($inner)* } );
                        )*
                    }

                )*
            })*
        }
    }
}

// TODO: Get these in the macro somehow

// Won't actually ever be serialized. Just used for the macro to be happy
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

packets! {
    Serverbound => {
        Handshaking => {
            0x00 => Handshake {
                protocol_version: v32,
                server_address: BoundedString<255>,
                server_port: u16,
                next_state: u8, // is technically a varint, but the valid range is within a u8
            }
        },
        Status => {
            0x00 => Request {},
            0x01 => Ping {
                payload: i64,
            }
        },
        Login => {
            0x00 => LoginStart {
                name: BoundedString<16>,
            },
            0x01 => EncryptionResponse {
                shared_secret_length: v32,
                shared_secret: Vec<u8, shared_secret_length>,
                verify_token_length: v32,
                verify_token: Vec<u8, verify_token_length>,
            }
        }
    },
    Clientbound => {
        Status => {
            0x00 => Response {
                json_response: BoundedString<32767>,
            },
            0x01 => Pong {
                payload: i64,
            }
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
            }
        }
    },
    Internal => {
        Client => {
        },
        Network => {
            0x00 => Disconnect {
                reason: Chat,
            }
        }
    }
}

impl serial::Decode for Packets {
    fn decode(_decoder: &mut serial::Decoder) -> Result<Self, serial::DecodeError> {
        panic!("Decode is not implemented for Packets");
    }
}
