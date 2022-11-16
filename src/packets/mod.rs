pub(crate) mod types;

use crate::config::BC_CONFIG;

macro_rules! packet {
    {@ $decoder:ident $param:ident $type:ty, none} => {
        let $param = <$type as bincode::Decode>::decode($decoder)?;
    };
    {@ $decoder:ident $param:ident $type:ty, remain} => {
        todo!("Implement remain dec");
    };
    {@ $decoder:ident $param:ident $type:ty, $length:ident} => {
        let mut $param = Vec::with_capacity(u32::from($length) as usize);
        for _ in 0..u32::from($length) {
            $param.push(<u8 as bincode::Decode>::decode($decoder)?);
        }
    };
    {@ $encoder:ident $self:ident $param:ident none} => {
        bincode::Encode::encode(&$self.$param, $encoder)?;
    };
    {@ $encoder:ident $self:ident $param:ident remain} => {
        todo!("Implement remain enc");
    };
    {@ $encoder:ident $self:ident $param:ident $length:ident} => {
        for b in &$self.$param {
            bincode::Encode::encode(b, $encoder)?;
        }
    };
    {@ $name:ident { } -> ($(pub $param:ident : $type:ty => $length:ident),* $(,)?)} => {
        pub struct $name {
            $(pub $param : $type),*
        }
        impl bincode::Decode for $name {
            fn decode<D: bincode::de::Decoder>(
                decoder: &mut D,
            ) -> core::result::Result<Self, bincode::error::DecodeError> {
                $(packet!{@ decoder $param $type, $length})*

                Ok(Self {
                    $($param),*
                })
            }
        }
        impl bincode::Encode for $name {
            fn encode<E: bincode::enc::Encoder>(
                &self,
                encoder: &mut E,
            ) -> core::result::Result<(), bincode::error::EncodeError> {
                $(packet!{@ encoder self $param $length})*

                Ok(())
            }
        }
    };
    {@ $name:ident { $param:ident : Bytes<remain>, $($rest:tt)* } -> ($($result:tt)*)} => {
        packet! {
            @ $name {
                $($rest)*
            } -> (
                $($result)*
                pub $param : Vec<u8> => remain,
            )
        }
    };
    {@ $name:ident { $param:ident : Bytes<$length:ident>, $($rest:tt)* } -> ($($result:tt)*)} => {
        packet! {
            @ $name {
                $($rest)*
            } -> (
                $($result)*
                pub $param : Vec<u8> => $length,
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
    ($name:ident { $( $param:ident : $type:tt $(<$inner:tt>)?, )* $(,)* }) => {
        packet! {
            @ $name { $($param : $type $(<$inner>)?,)* } -> ()
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
                        $($($(Self::[<$dir:camel $state:camel $name:camel>](packet) => bincode::encode_to_vec(&packet, BC_CONFIG).unwrap(),)*)*)*
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
                        use crate::config::BC_CONFIG;

                        #[allow(unused_imports)]
                        use crate::packets::types::*;

                        use log::error;

                        use super::super::Packets;

                        pub fn decode_packet(id: u8, data: Vec<u8>) -> Option<Packets> {
                            match id {
                                $(
                                    $id => {
                                        let (packet, _) = bincode::decode_from_slice::<$name, _>(data.as_slice(), BC_CONFIG).unwrap();
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
#[derive(bincode::Decode, bincode::Encode, Copy, Clone)]
pub enum PacketState {
    Handshake,
    Status,
    Login,
    Play,
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

impl From<PacketState> for u8 {
    fn from(state: PacketState) -> Self {
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
                shared_secret: Bytes<shared_secret_length>,
                verify_token_length: v32,
                verify_token: Bytes<verify_token_length>,
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
        }
    },
    Internal => {
        Client => {
        },
        Network => {
            0x00 => Disconnect {
                reason: BoundedString<32767>,
            }
        }
    }
}

impl bincode::Decode for Packets {
    fn decode<D: bincode::de::Decoder>(
        _decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        panic!("Decode is not implemented for Packets");
    }
}

impl<'de> bincode::BorrowDecode<'de> for Packets {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        _decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        panic!("BorrowDecode is not implemented for Packets");
    }
}
