use bincode::{BorrowDecode, Decode, Encode};
use log::debug;

use super::v32;

pub struct BoundedString<const L: usize> {
    value: String,
}

/// Serialization
impl<const L: usize> bincode::Encode for BoundedString<L> {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        debug!("Encoding BoundedString with length {}", self.value.len());
        Encode::encode(&v32::from(self.value.len() as u32), encoder)?;

        for byte in self.value.as_bytes() {
            Encode::encode(byte, encoder)?;
        }

        Ok(())
    }
}

impl<const L: usize> bincode::Decode for BoundedString<L> {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let len = u32::from(<v32 as Decode>::decode(decoder)?);

        // TODO: Check if there's a way to decode variable length arrays without length
        let mut bytes = Vec::<u8>::with_capacity(len as usize);
        for _ in 0..len {
            bytes.push(Decode::decode(decoder)?);
        }

        let value = String::from_utf8(bytes).map_err(|e| bincode::error::DecodeError::Utf8 {
            inner: e.utf8_error(),
        })?;
        Ok(BoundedString::<L>::from(value))
    }
}

impl<'de, const L: usize> BorrowDecode<'de> for BoundedString<L> {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let len = u32::from(<v32 as BorrowDecode>::borrow_decode(decoder)?);

        // TODO: Check if there's a way to decode variable length arrays without length
        let mut bytes = Vec::<u8>::with_capacity(len as usize);
        for _ in 0..len {
            bytes.push(BorrowDecode::borrow_decode(decoder)?);
        }

        let value = String::from_utf8(bytes).map_err(|e| bincode::error::DecodeError::Utf8 {
            inner: e.utf8_error(),
        })?;
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
