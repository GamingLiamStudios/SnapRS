use bincode::{BorrowDecode, Decode, Encode};

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct v32 {
    value: u32,
}

impl v32 {
    pub fn read_from_vec(vec: &[u8]) -> Self {
        let mut value = 0;

        for i in (0..32).step_by(7) {
            let b = vec[i / 7];
            value |= ((b & 0x7F) as u32) << i;

            if (b & 0x80) == 0 {
                return Self { value };
            }
        }

        panic!("Varint too big");
    }
}

/// Serialization
impl bincode::Encode for v32 {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        let mut num = self.value;
        loop {
            if (num & !0x7F) == 0 {
                Encode::encode(&(num as u8), encoder)?;
                break;
            }

            Encode::encode(&(((num & 0x7F) | 0x80) as u8), encoder)?;
            num >>= 7;
        }

        Ok(())
    }
}

impl bincode::Decode for v32 {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let mut value = 0;

        for i in (0..32).step_by(7) {
            let b: u8 = Decode::decode(decoder)?;
            value |= ((b & 0x7F) as u32) << i;

            if (b & 0x80) == 0 {
                return Ok(Self { value });
            }
        }
        Err(bincode::error::DecodeError::OtherString(
            "VarInt too big".to_string(),
        ))
    }
}

impl<'de> BorrowDecode<'de> for v32 {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let mut value = 0;

        for i in (0..32).step_by(7) {
            let b: u8 = BorrowDecode::borrow_decode(decoder)?;
            value |= ((b & 0x7F) as u32) << i;

            if (b & 0x80) == 0 {
                return Ok(Self { value });
            }
        }
        Err(bincode::error::DecodeError::OtherString(
            "VarInt too big".to_string(),
        ))
    }
}

/// Integer convertions
impl From<u32> for v32 {
    fn from(value: u32) -> Self {
        Self { value }
    }
}

impl From<i32> for v32 {
    fn from(value: i32) -> Self {
        Self {
            value: value as u32,
        }
    }
}

impl From<v32> for u32 {
    fn from(value: v32) -> Self {
        value.value
    }
}

impl From<v32> for i32 {
    fn from(value: v32) -> Self {
        value.value as i32
    }
}
