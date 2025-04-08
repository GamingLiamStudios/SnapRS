#![allow(clippy::ref_option_ref, clippy::used_underscore_binding)]
use nom::{
    IResult,
    Parser,
    bytes::tag,
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
use crate::{
    parser::{
        construct_varint,
        parse_string,
        parse_varint,
    },
    text::TextComponent,
};

pub mod client;

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

impl<'a> LoginPacket<'a> {
    fn start(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (data, username) = parse_string::<16>(data)?;
        let (data, uuid) = be_u128().parse(data)?;
        Ok((data, LoginPacket::Start {
            name: username,
            uuid: Uuid::from_u128(uuid),
        }))
    }

    fn encryption(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (data, secret) = length_data(parse_varint.map(i32::cast_unsigned)).parse(data)?;
        let (data, verify) = length_data(parse_varint.map(i32::cast_unsigned)).parse(data)?;
        Ok((data, Self::Encryption { secret, verify }))
    }

    fn plugin(data: &'a [u8]) -> IResult<&'a [u8], Self> {
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
    }

    #[allow(clippy::unnecessary_wraps)]
    const fn ack(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        Ok((data, Self::Ack))
    }
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
        nom::branch::alt((
            tag(&construct_varint::<0x00>()[..]).and(LoginPacket::start),
            tag(&construct_varint::<0x01>()[..]).and(LoginPacket::encryption),
            tag(&construct_varint::<0x02>()[..]).and(LoginPacket::plugin),
            tag(&construct_varint::<0x03>()[..]).and(LoginPacket::ack),
        ))
        .parse(data)
        .map(|(data, (_, packet))| (data, packet))
    }

    async fn handle(
        &mut self,
        packet: Self::PacketType<'_>,
    ) -> bool {
        match packet {
            LoginPacket::Ack => {
                unimplemented!("Configure state not yet implemented")
            },
            LoginPacket::Start { name, uuid } => {
                // TODO: Encryption
                // TODO: Compression

                // For testing, lets craft a disconnect packet
                let mut reason = TextComponent::new_text("Press ");
                reason.add_child(TextComponent::new_keybind("key.jump").bold().italic());
                reason.add_child(TextComponent::new_text(" to say \""));
                reason.add_child(TextComponent::new_text("Apple").bold());
                reason.add_child(TextComponent::new_text("\""));

                let packet = client::Disconnect { reason };
                self.stream
                    .write_packet(packet)
                    .await
                    .expect("Failed to send packet");
                true
            },
            _ => unimplemented!("Not yet implemented"),
        }
    }
}
