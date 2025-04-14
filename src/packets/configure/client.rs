use crab_nbt::NbtCompound;
use uuid::Uuid;

use super::ConfigureError;
use crate::{
    encode::{
        self,
        Generate,
    },
    packets::PacketBuilder,
    parser::IDENTIFIER_MAX_LEN,
    text::{
        self,
        TextComponent,
    },
};

#[derive(Debug)]
pub struct PluginMessage<'a> {
    pub channel: &'a str,
    pub data:    &'a [u8],
}

impl PacketBuilder<ConfigureError> for PluginMessage<'_> {
    const PACKET_ID: i32 = 0x00;
}

impl Generate<ConfigureError> for PluginMessage<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, ConfigureError> {
        if self.data.len() > 1_048_576 {
            return Err(ConfigureError::OversizedPluginData);
        }

        (
            encode::bounded_string::<{ IDENTIFIER_MAX_LEN as usize }, _>(self.channel),
            self.data,
        )
            .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct Disconnect<'a> {
    pub reason: &'a TextComponent,
}

impl PacketBuilder<ConfigureError> for Disconnect<'_> {
    const PACKET_ID: i32 = 0x01;
}

impl Generate<ConfigureError> for Disconnect<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, ConfigureError> {
        text::write_text_component(self.reason).generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct Finish;

impl PacketBuilder<ConfigureError> for Finish {
    const PACKET_ID: i32 = 0x02;
}

impl Generate<ConfigureError> for Finish {
    fn generate_in_place(
        &self,
        _buf: &mut Vec<u8>,
    ) -> Result<usize, ConfigureError> {
        Ok(0)
    }
}

#[derive(Debug)]
pub struct KeepAlive {
    keep_alive_id: i32,
}

impl PacketBuilder<ConfigureError> for KeepAlive {
    const PACKET_ID: i32 = 0x03;
}

impl Generate<ConfigureError> for KeepAlive {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, ConfigureError> {
        self.keep_alive_id.generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct Ping {
    id: i32,
}

impl PacketBuilder<ConfigureError> for Ping {
    const PACKET_ID: i32 = 0x04;
}

impl Generate<ConfigureError> for Ping {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, ConfigureError> {
        self.id.generate_in_place(buf)
    }
}

// TODO: Test this with actual registry data
#[derive(Debug)]
pub struct RegistryData {
    data: crab_nbt::Nbt,
}

impl RegistryData {
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: crab_nbt::Nbt::new("root".to_string(), NbtCompound::new()),
        }
    }

    // TODO: Allow this to have generic inputs that impl Serialize
    pub fn add_entry(
        &mut self,
        data: crab_nbt::Nbt,
    ) {
        self.data.root_tag.put(data.name, data.root_tag);
    }
}

impl PacketBuilder<ConfigureError> for RegistryData {
    const PACKET_ID: i32 = 0x05;
}

impl Generate<ConfigureError> for RegistryData {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, ConfigureError> {
        let before = buf.len();
        self.data.write_unnamed_to_writer(&mut *buf)?;
        Ok(buf.len() - before)
    }
}

#[derive(Debug)]
pub enum RemovePack {
    All,
    One(Uuid),
}

impl PacketBuilder<ConfigureError> for RemovePack {
    const PACKET_ID: i32 = 0x06;
}

impl Generate<ConfigureError> for RemovePack {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, ConfigureError> {
        match self {
            Self::All => {
                buf.push(0);
                Ok(1)
            },
            Self::One(uuid) => (1u8, uuid.as_u128()).generate_in_place(buf),
        }
    }
}

#[derive(Debug)]
pub struct AddPack<'a> {
    uuid: Uuid,

    url:  &'a str,
    hash: &'a str,

    forced: bool,
    prompt: Option<TextComponent>,
}

impl PacketBuilder<ConfigureError> for AddPack<'_> {
    const PACKET_ID: i32 = 0x07;
}

impl Generate<ConfigureError> for AddPack<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, ConfigureError> {
        (
            self.uuid.as_u128(),
            encode::bounded_string::<32767, _>(self.url),
            encode::bounded_string::<40, _>(self.hash),
            u8::from(self.forced),
            match self.prompt {
                None => 0u8,
                Some(_) => 1u8,
            },
            self.prompt.as_ref().map(text::write_text_component),
        )
            .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct FeatureFlags<'a> {
    enabled: Vec<&'a str>,
}

impl PacketBuilder<ConfigureError> for FeatureFlags<'_> {
    const PACKET_ID: i32 = 0x08;
}

impl Generate<ConfigureError> for FeatureFlags<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, ConfigureError> {
        let mut running_total = encode::write_varint::<ConfigureError>(
            i32::try_from(self.enabled.len()).expect("Too many FeatureFlags"),
        )
        .generate_in_place(buf)?;

        for flag in &self.enabled {
            running_total +=
                encode::bounded_string::<{ IDENTIFIER_MAX_LEN as usize }, ConfigureError>(flag)
                    .generate_in_place(buf)?;
        }

        Ok(running_total)
    }
}

// TODO: Update Tags
