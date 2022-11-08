pub(crate) mod types;

pub trait Packet: Send {
    fn get_id(&self) -> u8;
    fn get_data(&self) -> Vec<u8>;
}

macro_rules! packets {
    { $($dir:ident => { $($state:ident => { $($id:expr => $name:ident { $($inner:tt)* }),* }),* }),* } => {
        paste::paste! {
            #[allow(dead_code)]
            pub enum Packets {
                $($($([<$dir:camel $state:camel $name:camel>]([<$dir:lower>]::[<$state:lower _packets>]::$name),)*)*)*
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
                                        Some(Packets::[<$dir:camel $state:camel $name:camel>](packet))
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
pub enum PacketDirection {
    Clientbound,
    Serverbound,
}

// Won't actually ever be serialized. Just used for the macro
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
            0x01 => Ping {
                pub payload: i64
            }
        }
    },
    Internal => {
        Client => {
            0x00 => Disconnect {
                pub reason: BoundedString<32767>
            },
            0x01 => SwitchState {
                pub state: crate::packets::PacketState
            }
        },
        Network => {
            0x00 => Disconnect {
                pub reason: BoundedString<32767>
            },
            0x01 => SwitchState {
                pub state: crate::packets::PacketState
            }
        }
    }
}
