use nom::{
    IResult,
    Parser,
    multi::length_data,
    number::{
        be_u8,
        be_u128,
    },
};
use uuid::Uuid;

use super::{
    ClientConnection,
    StateParser,
};
use crate::parser::{
    parse_string,
    parse_varint,
};

fn fmt_plugin_response(
    v: &Option<&'_ [u8]>,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    f.write_str(if v.is_some() { "Exists" } else { "Ack" })
}

#[derive(educe::Educe)]
#[educe(Debug)]
pub enum LoginPacket<'a> {
    Start {
        name: &'a str,
        uuid: Uuid,
    },
    Encryption {
        #[educe(Debug(ignore))]
        secret: &'a [u8],
        #[educe(Debug(ignore))]
        verify: &'a [u8],
    },
    Plugin {
        id:   i32,
        #[educe(Debug(method(fmt_plugin_response)))]
        data: Option<&'a [u8]>,
    },
    Ack,
}

pub struct LoginClient {
    stream: ClientConnection,
}

impl LoginClient {
    pub const fn new(stream: ClientConnection) -> Self {
        Self { stream }
    }
}

impl StateParser for LoginClient {
    type PacketType<'a> = LoginPacket<'a>;

    fn stream(&mut self) -> &mut ClientConnection {
        &mut self.stream
    }

    fn parser(data: &[u8]) -> IResult<&[u8], Self::PacketType<'_>> {
        let (data, packet_id) = parse_varint(data)?;
        match packet_id {
            0x00 => {
                let (data, username) = parse_string::<16>(data)?;
                let (data, uuid) = be_u128().parse(data)?;
                Ok((data, LoginPacket::Start {
                    name: username,
                    uuid: Uuid::from_u128(uuid),
                }))
            },
            0x01 => {
                let (data, (secret, verify)) = (
                    length_data(parse_varint.map(i32::cast_unsigned)),
                    length_data(parse_varint.map(i32::cast_unsigned)),
                )
                    .parse(data)?;
                Ok((data, LoginPacket::Encryption { secret, verify }))
            },
            0x02 => {
                let (data, id) = parse_varint(data)?;
                let (data, succeeded) = be_u8().parse(data)?;
                assert!(
                    data.len() <= 1_048_576,
                    "Plugin Data Response is larger than expected"
                );
                Ok((data, LoginPacket::Plugin {
                    id,
                    data: if succeeded != 0 { Some(data) } else { None },
                }))
            },
            0x03 => Ok((data, LoginPacket::Ack)),
            _ => unimplemented!("No other packets exist for Status"),
        }
    }

    async fn handle(
        &mut self,
        packet: Self::PacketType<'_>,
    ) -> bool {
        unimplemented!()
    }
}
