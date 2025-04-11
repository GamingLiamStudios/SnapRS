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
