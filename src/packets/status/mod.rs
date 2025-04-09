use nom::{
    IResult,
    Parser,
    bytes::tag,
    number::be_i64,
};

use super::{
    ClientConnection,
    DummyError,
    NextState,
    StateParser,
};
use crate::{
    parser::construct_varint,
    text::TextComponent,
};

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

pub struct StatusClient {}

impl StatusClient {
    pub const fn new() -> Self {
        Self {}
    }
}

impl StateParser for StatusClient {
    type Error = DummyError;
    type PacketType<'a> = StatusPacket;

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
        client: &mut ClientConnection,
        packet: Self::PacketType<'_>,
    ) -> Result<NextState, Self::Error> {
        match packet {
            StatusPacket::Request => {
                // Send Response packet
                client.write_packet(client::StatusResponse {}).await?;
                Ok(NextState::None)
            },
            StatusPacket::Ping(timestamp) => {
                // Send pong packet
                client
                    .write_packet(client::StatusPong { timestamp })
                    .await?;
                Err(DummyError)
            },
        }
    }

    async fn handle_disconnect(
        &mut self,
        _client: &mut ClientConnection,
        _reason: &TextComponent,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
