use crate::packets::serial;

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct v32 {
    value: u32,
}

impl v32 {
    pub fn read_from_slice(slice: &[u8]) -> (Self, usize) {
        let mut value = 0;

        for i in (0..32).step_by(7) {
            if slice.len() <= i / 7 {
                panic!("Not enough bytes to read v32");
            }
            let b = slice[i / 7];
            value |= ((b & 0x7F) as u32) << i;

            if (b & 0x80) == 0 {
                return (Self { value }, (i / 7) + 1);
            }
        }

        panic!("Varint too big");
    }

    pub fn byte_size(val: u32) -> usize {
        match val {
            0..=0x7F => 1,
            0x80..=0x3FFF => 2,
            0x4000..=0x1FFFFF => 3,
            0x200000..=0xFFFFFFF => 4,
            _ => 5,
        }
    }
}

/// Serialization
impl serial::Encode for v32 {
    fn encode(&self, encoder: &mut serial::Encoder) -> Result<(), serial::EncodeError> {
        let mut num = self.value;
        loop {
            if (num & !0x7F) == 0 {
                serial::Encode::encode(&(num as u8), encoder)?;
                break;
            }

            serial::Encode::encode(&(((num & 0x7F) | 0x80) as u8), encoder)?;
            num >>= 7;
        }

        Ok(())
    }
}

impl serial::Decode for v32 {
    fn decode(decoder: &mut serial::Decoder) -> Result<Self, serial::DecodeError> {
        let mut value = 0;

        for i in (0..32).step_by(7) {
            let b: u8 = serial::Decode::decode(decoder)?;
            value |= ((b & 0x7F) as u32) << i;

            if (b & 0x80) == 0 {
                return Ok(Self { value });
            }
        }
        Err(serial::DecodeError::InvalidData)
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
