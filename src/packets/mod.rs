pub(crate) mod types;

use crate::config::BC_CONFIG;

pub trait Packet {
    fn get_id(&self) -> u8;
    fn get_data(&self) -> Vec<u8>;
}

macro_rules! packets {
    { $($dir:ident => { $($state:ident => { $($id:expr => $name:ident { $($inner:tt)* }),* }),* }),* } => {
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
                        $($($(Self::[<$dir:camel $state:camel $name:camel>](..) => write!(f, "[<$dir:camel $state:camel $name:camel>]").unwrap(),)*)*)*
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
                        use crate::packets::{Packet, types::*};

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
                            #[derive(bincode::Decode, bincode::Encode)]
                            pub struct $name {
                                $($inner)*
                            }

                            impl Packet for $name {
                                fn get_id(&self) -> u8 {
                                    $id
                                }
                                fn get_data(&self) -> Vec<u8> {
                                    bincode::encode_to_vec(self, BC_CONFIG).unwrap()
                                }
                            }
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
                pub protocol_version: v32,
                pub server_address: BoundedString<255>,
                pub server_port: u16,
                pub next_state: u8, // is technically a varint, but the valid range is within a u8
            }
        },
        Status => {
            0x00 => Request {},
            0x01 => Ping {
                pub payload: i64
            }
        }
    },
    Clientbound => {
        Status => {
            0x00 => Response {
                pub json_response: BoundedString<32767>
            },
            0x01 => Pong {
                pub payload: i64
            }
        }
    },
    Internal => {
        Client => {
        },
        Network => {
            0x00 => Disconnect {
                pub reason: BoundedString<32767>
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
