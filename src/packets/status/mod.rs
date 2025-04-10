use nom::{
    IResult,
    Parser,
    bytes::tag,
    number::be_i64,
};

use crate::parser::construct_varint;

pub mod client;

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

    pub fn parse(data: &[u8]) -> IResult<&[u8], Self> {
        nom::branch::alt((
            tag(&construct_varint::<0x00>()[..]).and(Self::request),
            tag(&construct_varint::<0x01>()[..]).and(Self::ping),
        ))
        .parse(data)
        .map(|(data, (_, packet))| (data, packet))
    }
}
