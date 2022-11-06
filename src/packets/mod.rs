pub(crate) mod types;

pub trait Packet: Send {
    fn get_id(&self) -> u8;
    fn get_data(&self) -> Vec<u8>;
}

pub struct PacketResult {
    packet: Box<dyn Packet>,
    read: usize,
}

macro_rules! packets {
    { $($dir:ident => { $($state:ident => { $($id:expr => $name:ident { $($inner:tt)* }),* }),* }),* } => {
        paste::paste! {
            $(pub mod $dir {
                $(
                    pub use [<$state _packets>]::decode_packet as [<decode_ $state>];
                    mod [<$state _packets>] {
                        use bincode::{Decode, Encode};
                        use crate::config::BC_CONFIG;
                        use crate::packets::{Packet, PacketResult, types::*};

                        use log::error;

                        pub fn decode_packet(id: u8, data: Vec<u8>) -> Option<PacketResult> {
                            match id {
                                $(
                                    $id => {
                                        let t = bincode::decode_from_slice::<$name, _>(data.as_slice(), BC_CONFIG).unwrap();
                                        Some(PacketResult {packet: Box::new(t.0), read: t.1})
                                    },
                                )*
                                _ => {
                                    error!("Unknown packet id: {}", id);
                                    None
                                },
                            }
                        }
                        $(
                            #[derive(Decode, Encode)]
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

packets! {
    serverbound => {
        handshaking => {
            0x00 => Handshake {
                protocol_version: v32,
                server_address: BoundedString<255>,
                server_port: u16,
                next_state: u8, // is technically a varint, but the valid range is within a u8
            }
        },
        status => {
            0x00 => Request {},
            0x01 => Ping {
                payload: i64
            }
        }
    }
}
