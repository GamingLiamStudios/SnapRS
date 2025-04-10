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

use super::PacketError;
use crate::{
    encode::EncodeError,
    parser::{
        construct_varint,
        parse_string,
        parse_varint,
    },
    text::TextComponent,
};

pub mod client;

#[derive(Debug)]
pub enum LoginError {
    OversizedPluginData,
    Encode(EncodeError),
    Io(std::io::Error),
}

impl From<std::io::Error> for LoginError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<EncodeError> for LoginError {
    fn from(value: EncodeError) -> Self {
        Self::Encode(value)
    }
}

impl PacketError for LoginError {
    fn describe(&self) -> TextComponent {
        match self {
            Self::OversizedPluginData => {
                TextComponent::new_text("Server attempted to send oversize Plugin Payload")
            },
            Self::Encode(error) => error.describe(),
            Self::Io(error) => TextComponent::new_text(error.to_string()),
        }
    }
}

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
        if data.len() > 1_048_576 {
            Err(nom::Err::Error(nom::error::Error::new(
                data,
                nom::error::ErrorKind::TooLarge,
            )))
        } else {
            Ok((data, LoginPacket::Plugin {
                id,
                data: if succeeded != 0 { Some(data) } else { None },
            }))
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    const fn ack(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        Ok((data, Self::Ack))
    }

    pub fn parse(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        nom::branch::alt((
            tag(&construct_varint::<0x00>()[..]).and(Self::start),
            tag(&construct_varint::<0x01>()[..]).and(Self::encryption),
            tag(&construct_varint::<0x02>()[..]).and(Self::plugin),
            tag(&construct_varint::<0x03>()[..]).and(Self::ack),
        ))
        .parse(data)
        .map(|(data, (_, packet))| (data, packet))
    }
}
