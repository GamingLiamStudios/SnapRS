use uuid::Uuid;

use crate::{
    encode::{
        self,
        Generate,
    },
    packets::{
        PacketBuilder,
        PacketError,
    },
    text::{
        self,
        TextComponent,
    },
};

#[derive(Debug)]
pub struct Disconnect {
    pub reason: TextComponent,
}

impl PacketBuilder for Disconnect {
    const PACKET_ID: i32 = 0x00;
}

impl Generate<PacketError> for Disconnect {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PacketError> {
        text::write_json_text_component(&self.reason).generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct EncryptionRequest<'a> {
    /// 1024b RSA pubkey, packaged in ASN.1 DER
    pub key: &'a [u8],

    /// typically 4 bytes
    pub verify: &'a [u8],
}

impl PacketBuilder for EncryptionRequest<'_> {
    const PACKET_ID: i32 = 0x01;
}

impl Generate<PacketError> for EncryptionRequest<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PacketError> {
        (
            encode::bounded_string::<20, _>(""),
            encode::length_value(self.key, |length| {
                encode::write_varint(
                    i32::try_from(length).expect("Length of Packet was larger than i32::MAX"),
                )
            }),
            encode::length_value(self.verify, |length| {
                encode::write_varint(
                    i32::try_from(length).expect("Length of Packet was larger than i32::MAX"),
                )
            }),
        )
            .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct Property<'a> {
    pub name:      &'a str,
    pub value:     &'a str,
    pub signature: Option<&'a str>,
}

impl Generate<PacketError> for Property<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PacketError> {
        (
            encode::bounded_string::<32_767, _>(self.name),
            encode::bounded_string::<32_767, _>(self.value),
            self.signature.map(encode::bounded_string::<32_767, _>),
        )
            .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct Success<'a> {
    pub uuid:       Uuid,
    pub username:   &'a str,
    pub properties: Vec<Property<'a>>,
}

impl PacketBuilder for Success<'_> {
    const PACKET_ID: i32 = 0x02;
}

impl Generate<PacketError> for Success<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PacketError> {
        (
            self.uuid.as_u128(),
            encode::bounded_string::<16, _>(self.username),
            encode::length_value(self.properties.as_slice(), |length| {
                encode::write_varint(
                    i32::try_from(length).expect("Length of Packet was larger than i32::MAX"),
                )
            }),
        )
            .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct Compression {
    pub max_size: i32,
}

impl PacketBuilder for Compression {
    const PACKET_ID: i32 = 0x03;
}

impl Generate<PacketError> for Compression {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PacketError> {
        encode::write_varint(self.max_size).generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct PluginRequest<'a> {
    pub message_id: i32,
    pub channel:    &'a str,
    pub data:       &'a [u8],
}

impl PacketBuilder for PluginRequest<'_> {
    const PACKET_ID: i32 = 0x04;
}

impl Generate<PacketError> for PluginRequest<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PacketError> {
        assert!(
            self.data.len() <= 1_048_576,
            "Data is larger than Notchian Client supports"
        );
        (
            encode::write_varint(self.message_id),
            encode::bounded_string::<32767, _>(self.channel),
            self.data,
        )
            .generate_in_place(buf)
    }
}
