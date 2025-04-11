use nom::{
    IResult,
    Parser,
    branch::alt,
    combinator::value,
    number::{
        be_f32,
        be_f64,
        be_i8,
        be_i32,
        be_i64,
        be_u8,
    },
    sequence::pair,
};

use super::{
    ChatMode,
    EnabledSkinParts,
    MainHand,
};
use crate::parser::{
    IDENTIFIER_MAX_LEN,
    parse_bool,
    parse_string,
    parse_varint,
    tag_varint,
};

pub mod client;

#[derive(Debug)]
pub enum PlayPacket<'a> {
    ConfirmTeleport(i32),
    ClientInformation {
        locale:        &'a str,
        view_distance: i8,

        chat_mode:   ChatMode,
        chat_colors: bool,

        enabled_skin: EnabledSkinParts,
        main_hand:    MainHand,

        text_filtering:  bool,
        server_listings: bool,
    },
    ChatMessage {
        message:   &'a str,
        timestamp: i64,

        salt:      i64,
        signature: Option<&'a [u8]>,

        message_count: i32,
        acknowledged:  [u8; 3], // 20 bits
    },
    AckConfig,
    PluginMessage {
        channel: &'a str,
        data:    &'a [u8],
    },
    PlayerPosition {
        x: f64,
        y: f64,
        z: f64,

        grounded: bool,
    },
    PlayerRotation {
        yaw:   f32,
        pitch: f32,

        grounded: bool,
    },
    PlayerPositionRotation {
        x: f64,
        y: f64,
        z: f64,

        yaw:   f32,
        pitch: f32,

        grounded: bool,
    },
    OnGround(bool),
    RequestPing(i64),
    Pong(i32),
    KeepAlive(i64),
    ChunkBatch(f32),
    Unimplemented {
        id:   i32,
        data: &'a [u8],
    },
}

impl<'a> PlayPacket<'a> {
    fn confirm_teleport(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        parse_varint.map(Self::ConfirmTeleport).parse(data)
    }

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
                value(ChatMode::Enabled, tag_varint::<0>),
                value(ChatMode::CommandsOnly, tag_varint::<1>),
                value(ChatMode::Disabled, tag_varint::<2>),
            )),
            parse_bool,
            be_u8().map(|v| EnabledSkinParts::from_bits(v).expect("Failed to load from bits")),
            alt((
                value(MainHand::Left, tag_varint::<0>),
                value(MainHand::Right, tag_varint::<1>),
            )),
            parse_bool,
            parse_bool,
        )
            .parse(data)?;

        Ok((data, Self::ClientInformation {
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
    const fn ack_config(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        Ok((data, Self::AckConfig))
    }

    // TODO: Chat Message

    fn request_ping(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        be_i64().map(Self::RequestPing).parse(data)
    }

    fn keep_alive(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        be_i64().map(Self::KeepAlive).parse(data)
    }

    fn pong(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        be_i32().map(Self::Pong).parse(data)
    }

    fn chunk_batch(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        be_f32().map(Self::ChunkBatch).parse(data)
    }

    fn player_position(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (data, (x, y, z, grounded)) = (be_f64(), be_f64(), be_f64(), parse_bool).parse(data)?;
        Ok((data, Self::PlayerPosition { x, y, z, grounded }))
    }

    fn player_rotation(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (data, (yaw, pitch, grounded)) = (be_f32(), be_f32(), parse_bool).parse(data)?;
        Ok((data, Self::PlayerRotation {
            yaw,
            pitch,
            grounded,
        }))
    }

    fn player_position_rotation(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (data, (x, y, z, yaw, pitch, grounded)) =
            (be_f64(), be_f64(), be_f64(), be_f32(), be_f32(), parse_bool).parse(data)?;
        Ok((data, Self::PlayerPositionRotation {
            x,
            y,
            z,
            yaw,
            pitch,
            grounded,
        }))
    }

    fn on_ground(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        parse_bool.map(Self::OnGround).parse(data)
    }

    fn unimplemented(data: &'a [u8]) -> IResult<&'a [u8], (&'a [u8], Self)> {
        let (data, id) = parse_varint(data)?;
        Ok((&[], (&[], Self::Unimplemented { id, data })))
    }

    pub fn parse(data: &'a [u8]) -> IResult<&'a [u8], Self> {
        alt((
            pair(tag_varint::<0x00>, Self::confirm_teleport),
            //pair(tag_varint::<0x05>, Self::chat_message),
            pair(tag_varint::<0x07>, Self::chunk_batch),
            pair(tag_varint::<0x09>, Self::information),
            pair(tag_varint::<0x0b>, Self::ack_config),
            pair(tag_varint::<0x10>, Self::plugin_message),
            pair(tag_varint::<0x15>, Self::keep_alive),
            pair(tag_varint::<0x17>, Self::player_position),
            pair(tag_varint::<0x18>, Self::player_position_rotation),
            pair(tag_varint::<0x19>, Self::player_rotation),
            pair(tag_varint::<0x1a>, Self::on_ground),
            pair(tag_varint::<0x1e>, Self::request_ping),
            pair(tag_varint::<0x24>, Self::pong),
            Self::unimplemented,
        ))
        .map(|(_, packet)| packet)
        .parse(data)
    }
}
