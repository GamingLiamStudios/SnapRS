#[derive(Debug)]
pub enum DecodeError {
    NotEnoughBytes,
    InvalidData,
}

pub trait Decode {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError>
    where
        Self: Sized;
}

pub struct Decoder<'a> {
    pub(self) buffer: &'a [u8],
    pub(self) offset: usize,
}

impl Decoder<'_> {
    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.offset
    }
}

pub fn decode_from_slice<R: Decode>(buffer: &[u8]) -> Result<(R, usize), DecodeError> {
    let mut decoder = Decoder { buffer, offset: 0 };
    Ok((<R as Decode>::decode(&mut decoder)?, decoder.offset)) // Hopefully this executes in the correct order.
}

impl Decode for u8 {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        if decoder.remaining() < 1 {
            return Err(DecodeError::NotEnoughBytes);
        }

        let value = decoder.buffer[decoder.offset];
        decoder.offset += 1;
        Ok(value)
    }
}

impl Decode for u16 {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        if decoder.remaining() < 2 {
            return Err(DecodeError::NotEnoughBytes);
        }

        let value = u16::from_be_bytes(
            decoder.buffer[decoder.offset..decoder.offset + 2]
                .try_into()
                .unwrap(),
        );
        decoder.offset += 2;
        Ok(value)
    }
}

impl Decode for u32 {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        if decoder.remaining() < 4 {
            return Err(DecodeError::NotEnoughBytes);
        }

        let value = u32::from_be_bytes(
            decoder.buffer[decoder.offset..decoder.offset + 4]
                .try_into()
                .unwrap(),
        );
        decoder.offset += 4;
        Ok(value)
    }
}

impl Decode for u64 {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        if decoder.remaining() < 8 {
            return Err(DecodeError::NotEnoughBytes);
        }

        let value = u64::from_be_bytes(
            decoder.buffer[decoder.offset..decoder.offset + 8]
                .try_into()
                .unwrap(),
        );
        decoder.offset += 8;
        Ok(value)
    }
}

impl Decode for i8 {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        Ok(<u8 as Decode>::decode(decoder)? as i8)
    }
}

impl Decode for i16 {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        Ok(<u16 as Decode>::decode(decoder)? as i16)
    }
}

impl Decode for i32 {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        Ok(<u32 as Decode>::decode(decoder)? as i32)
    }
}

impl Decode for i64 {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        Ok(<u64 as Decode>::decode(decoder)? as i64)
    }
}

impl Decode for f32 {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        Ok(f32::from_bits(<u32 as Decode>::decode(decoder)?))
    }
}

impl Decode for f64 {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        Ok(f64::from_bits(<u64 as Decode>::decode(decoder)?))
    }
}

impl Decode for bool {
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        Ok(u8::decode(decoder)? != 0)
    }
}
