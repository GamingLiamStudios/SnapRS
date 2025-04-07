use nom::{
    IResult,
    Parser,
    number::be_u16,
};
use tracing::{
    debug,
    info,
};

use super::{
    ClientConnection,
    StateParser,
};
use crate::{
    packets::{
        login::LoginClient,
        status::StatusClient,
    },
    parser::{
        parse_string,
        parse_varint,
    },
};

#[derive(Debug)]
pub enum NextState {
    Status,
    Login,
    Transfer,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum HandshakePacket<'a> {
    Intention {
        protocol_version: i32,
        server_address:   &'a str,
        server_port:      u16,
        next_state:       NextState,
    },
    LegacyPing {
        // TODO
    },
}

pub struct HandshakingClient {
    stream: ClientConnection,
}

impl HandshakingClient {
    pub const fn new(stream: ClientConnection) -> Self {
        Self { stream }
    }
}

impl StateParser for HandshakingClient {
    type PacketType<'a> = HandshakePacket<'a>;

    fn stream(&mut self) -> &mut ClientConnection {
        &mut self.stream
    }

    fn parser(data: &[u8]) -> IResult<&[u8], Self::PacketType<'_>> {
        let (data, packet_id) = parse_varint(data)?;
        match packet_id {
            0x00 => {
                let (data, (protocol_version, server_address, server_port, next)) =
                    (parse_varint, parse_string::<256>, be_u16(), parse_varint).parse(data)?;
                Ok((data, HandshakePacket::Intention {
                    protocol_version,
                    server_address,
                    server_port,
                    next_state: match next {
                        1 => NextState::Status,
                        2 => NextState::Login,
                        3 => NextState::Transfer,
                        _ => unreachable!("Further states do not exist"),
                    },
                }))
            },
            _ => unimplemented!("No other states exist for Packet Handshake"),
        }
    }

    async fn handle(
        &mut self,
        packet: Self::PacketType<'_>,
    ) -> bool {
        match packet {
            HandshakePacket::Intention {
                protocol_version,
                server_address: _,
                server_port: _,
                next_state,
            } => {
                if protocol_version != 765 {
                    debug!(
                        protocol_version,
                        "Attempted connection from incompatible client"
                    );
                    return false; // Quit the client
                }

                match next_state {
                    NextState::Status => {
                        // Enter status flow
                        info!("Entering Status netflow");
                        StatusClient::new(self.stream.clone()).listen().await;
                    },
                    NextState::Login => {
                        // Enter login flow
                        info!("Entering Login netflow");
                        LoginClient::new(self.stream.clone()).listen().await;
                    },
                    NextState::Transfer => {
                        unimplemented!("i honestly have no idea what this is");
                    },
                }
            },
            HandshakePacket::LegacyPing {} => {
                unimplemented!("Legacy ping unimplemented");
            },
        }

        true
    }
}
