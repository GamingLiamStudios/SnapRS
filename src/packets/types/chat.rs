use super::BoundedString;

use crate::packets::serial;

// TODO
pub struct Chat {
    pub value: BoundedString<262144>,
}

impl serial::Encode for Chat {
    fn encode(&self, encoder: &mut serial::Encoder) -> Result<(), serial::EncodeError> {
        serial::Encode::encode(&self.value, encoder)?;
        Ok(())
    }
}

impl serial::Decode for Chat {
    fn decode(decoder: &mut serial::Decoder) -> Result<Self, serial::DecodeError> {
        let value = serial::Decode::decode(decoder)?;
        Ok(Self { value })
    }
}

impl From<String> for Chat {
    fn from(value: String) -> Self {
        if value.len() > 262144 {
            panic!("Chat string too long");
        }
        Self {
            // BoundedString::from WILL panic on longer chat strings. This is by design.
            value: BoundedString { value },
        }
    }
}
