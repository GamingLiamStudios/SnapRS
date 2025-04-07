use nom::{
    IResult,
    Parser,
    number::be_i64,
};

use super::{
    ClientConnection,
    StateParser,
};
use crate::parser::parse_varint;

mod client;

#[derive(Debug)]
#[allow(dead_code)]
pub enum StatusPacket {
    Request,
    Ping(i64),
}

pub struct StatusClient {
    stream: ClientConnection,
}

impl StatusClient {
    pub const fn new(stream: ClientConnection) -> Self {
        Self { stream }
    }
}

impl StateParser for StatusClient {
    type PacketType<'a> = StatusPacket;

    fn stream(&mut self) -> &mut ClientConnection {
        &mut self.stream
    }

    fn parser(data: &[u8]) -> IResult<&[u8], Self::PacketType<'_>> {
        let (data, packet_id) = parse_varint(data)?;
        match packet_id {
            0x00 => Ok((data, StatusPacket::Request)),
            0x01 => {
                let (data, timestamp) = be_i64().parse(data)?;
                Ok((data, StatusPacket::Ping(timestamp)))
            },
            _ => unimplemented!("No other packets exist for Status"),
        }
    }

    async fn handle(
        &mut self,
        packet: Self::PacketType<'_>,
    ) -> bool {
        match packet {
            StatusPacket::Request => {
                // Send Response packet
                _ = self.stream.write_packet(client::StatusResponse {}).await;
                false
            },
            StatusPacket::Ping(timestamp) => {
                // Send pong packet
                _ = self
                    .stream
                    .write_packet(client::StatusPong { timestamp })
                    .await;
                true
            },
        }
    }
}
