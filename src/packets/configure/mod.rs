use bitflags::bitflags;
use nom::{
    IResult,
    Parser,
    branch::alt,
    bytes::tag,
    combinator::value,
    number::{
        be_i8,
        be_i32,
        be_i64,
        be_u8,
        be_u128,
    },
    sequence::pair,
};
use uuid::Uuid;

use super::PacketError;
use crate::{
    encode::EncodeError,
    parser::{
        IDENTIFIER_MAX_LEN,
        construct_varint,
        parse_bool,
        parse_string,
        tag_varint,
    },
    text::TextComponent,
};

pub mod client;

#[derive(Debug)]
pub enum ConfigureError {
    OversizedPluginData,
    NbtEncode(crab_nbt::error::Error),
    Encode(EncodeError),
    Io(std::io::Error),
}

impl From<std::io::Error> for ConfigureError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<crab_nbt::error::Error> for ConfigureError {
    fn from(value: crab_nbt::error::Error) -> Self {
        Self::NbtEncode(value)
    }
}

impl From<EncodeError> for ConfigureError {
    fn from(value: EncodeError) -> Self {
        Self::Encode(value)
    }
}

impl PacketError for ConfigureError {
    fn describe(&self) -> TextComponent {
        match self {
            Self::OversizedPluginData => {
                TextComponent::new_text("Server attempted to send oversize Plugin Payload")
            },
            Self::NbtEncode(error) => TextComponent::new_text(error.to_string()),
            Self::Encode(error) => error.describe(),
            Self::Io(error) => TextComponent::new_text(error.to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ChatMode {
    Enabled,
    CommandsOnly,
    Disabled,
}

#[derive(Debug, Clone, Copy)]
pub enum MainHand {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum ClientResouceResponse {
    DownloadSuccess,
    Declined,
    DownloadFailed,
    Downloaded,
    Accepted,
    InvalidURL,
    ReloadFail,
    Discard,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct EnabledSkinParts: u8 {
        const Cape          = 1 << 0;
        const Jacked        = 1 << 1;
        const LeftSleeve    = 1 << 2;
        const RightSleeve   = 1 << 3;
        const LeftLeg       = 1 << 4;
        const RightLeg      = 1 << 5;
        const Hat           = 1 << 6;
    }
}

#[derive(Debug)]
pub enum ConfigurePacket<'a> {
    Information {
        locale:        &'a str,
        view_distance: i8,

        chat_mode:   ChatMode,
        chat_colors: bool,

        enabled_skin: EnabledSkinParts,
        main_hand:    MainHand,

        text_filtering:  bool,
        server_listings: bool,
    },
    PluginMessage {
        channel: &'a str,
        data:    &'a [u8],
    },
    AckFinish,
    KeepAlive {
        keep_alive_id: i64,
    },
    Pong {
        ping_id: i32,
    },
    ResourcePack {
        uuid:   Uuid,
        result: ClientResouceResponse,
    },
}

impl<'a> ConfigurePacket<'a> {
    fn information(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (
            data,
            (
                locale,
                view_distance,
                chat_mode,
                chat_colors,
                enabled_skin,
                main_hand,
                text_filtering,
                server_listings,
            ),
        ) = (
            parse_string::<16>,
            be_i8(),
            alt((
                value(ChatMode::Enabled, tag(&construct_varint::<0>()[..])),
                value(ChatMode::CommandsOnly, tag(&construct_varint::<1>()[..])),
                value(ChatMode::Disabled, tag(&construct_varint::<2>()[..])),
            )),
            parse_bool,
            be_u8().map(|v| EnabledSkinParts::from_bits(v).expect("Failed to load from bits")),
            alt((
                value(MainHand::Left, tag(&construct_varint::<0>()[..])),
                value(MainHand::Right, tag(&construct_varint::<1>()[..])),
            )),
            parse_bool,
            parse_bool,
        )
            .parse(data)?;

        Ok((data, Self::Information {
            locale,
            view_distance,
            chat_mode,
            chat_colors,
            enabled_skin,
            main_hand,
            text_filtering,
            server_listings,
        }))
    }

    fn plugin_message(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (data, channel) = parse_string::<IDENTIFIER_MAX_LEN>(data)?;
        Ok((&[], Self::PluginMessage { channel, data }))
    }

    #[allow(clippy::unnecessary_wraps)]
    const fn ack_finish(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        Ok((data, Self::AckFinish))
    }

    fn keep_alive(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (data, keep_alive_id) = be_i64().parse(data)?;
        Ok((data, Self::KeepAlive { keep_alive_id }))
    }

    fn pong(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (data, ping_id) = be_i32().parse(data)?;
        Ok((data, Self::Pong { ping_id }))
    }

    fn resource_pack(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (data, (uuid, result)) = (
            be_u128().map(Uuid::from_u128),
            alt((
                value(
                    ClientResouceResponse::DownloadSuccess,
                    tag(&construct_varint::<0>()[..]),
                ),
                value(
                    ClientResouceResponse::Declined,
                    tag(&construct_varint::<1>()[..]),
                ),
                value(
                    ClientResouceResponse::DownloadFailed,
                    tag(&construct_varint::<2>()[..]),
                ),
                value(
                    ClientResouceResponse::Accepted,
                    tag(&construct_varint::<3>()[..]),
                ),
                value(
                    ClientResouceResponse::Downloaded,
                    tag(&construct_varint::<4>()[..]),
                ),
                value(
                    ClientResouceResponse::InvalidURL,
                    tag(&construct_varint::<5>()[..]),
                ),
                value(
                    ClientResouceResponse::ReloadFail,
                    tag(&construct_varint::<6>()[..]),
                ),
                value(
                    ClientResouceResponse::Discard,
                    tag(&construct_varint::<7>()[..]),
                ),
            )),
        )
            .parse(data)?;
        Ok((data, Self::ResourcePack { uuid, result }))
    }

    pub fn parse(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        alt((
            pair(tag_varint::<0x00>, Self::information),
            pair(tag_varint::<0x01>, Self::plugin_message),
            pair(tag_varint::<0x02>, Self::ack_finish),
            pair(tag_varint::<0x03>, Self::keep_alive),
            pair(tag_varint::<0x04>, Self::pong),
            pair(tag_varint::<0x05>, Self::resource_pack),
        ))
        .map(|(_, packet)| packet)
        .parse(data)
    }
}
