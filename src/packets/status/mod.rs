use nom::{
    IResult,
    Parser,
    bytes::streaming::tag,
    number::be_i64,
};

use super::{
    ClientConnection,
    StateParser,
};
use crate::parser::construct_varint;

mod client;

#[derive(Debug)]
#[allow(dead_code)]
pub enum StatusPacket {
    Request,
    Ping(i64),
}

impl StatusPacket {
    #[allow(clippy::unnecessary_wraps)]
    const fn request(data: &[u8]) -> IResult<&[u8], Self> {
        Ok((data, Self::Request))
    }

    fn ping(data: &[u8]) -> IResult<&[u8], Self> {
        let (data, timestamp) = be_i64().parse(data)?;
        Ok((data, Self::Ping(timestamp)))
    }
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
        nom::branch::alt((
            tag(&construct_varint::<0x00>()[..]).and(StatusPacket::request),
            tag(&construct_varint::<0x01>()[..]).and(StatusPacket::ping),
        ))
        .parse(data)
        .map(|(data, (_, packet))| (data, packet))
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
