use crate::packets::serial;

use super::v32;

pub struct BoundedString<const L: usize> {
    pub(super) value: String,
}

/// Serialization
impl<const L: usize> serial::Encode for BoundedString<L> {
    fn encode(&self, encoder: &mut serial::Encoder) -> Result<(), serial::EncodeError> {
        serial::Encode::encode(&v32::from(self.value.len() as u32), encoder)?;

        for byte in self.value.as_bytes() {
            serial::Encode::encode(byte, encoder)?;
        }

        Ok(())
    }
}

impl<const L: usize> serial::Decode for BoundedString<L> {
    fn decode(decoder: &mut serial::Decoder) -> Result<Self, serial::DecodeError> {
        let len = u32::from(<v32 as serial::Decode>::decode(decoder)?);

        // TODO: Check if there's a way to decode variable length arrays without length
        let mut bytes = Vec::<u8>::with_capacity(len as usize);
        for _ in 0..len {
            bytes.push(serial::Decode::decode(decoder)?);
        }

        let value = String::from_utf8(bytes).map_err(|_| serial::DecodeError::InvalidData)?; // Should be fine?
        Ok(BoundedString::<L>::from(value))
    }
}

/// String constraining
impl<const L: usize> From<String> for BoundedString<{ L }> {
    fn from(value: String) -> Self {
        assert!(L > 0 && L <= 32767, "BoundedString outside of type-bounds");
        assert!(
            value.chars().count() <= L,
            "String outside specified bounds"
        );
        Self { value }
    }
}

impl<const L: usize> From<BoundedString<L>> for String {
    fn from(value: BoundedString<L>) -> Self {
        value.value
    }
}

/// Output
impl<const L: usize> std::fmt::Display for BoundedString<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}
