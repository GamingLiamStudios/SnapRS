#[derive(Debug)]
pub enum EncodeError {}

pub trait Encode {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError>;
}

pub struct Encoder {
    buffer: Vec<u8>,
}

pub fn encode_to_vec<E: Encode>(encode: &E) -> Result<Vec<u8>, EncodeError> {
    let mut encoder = Encoder { buffer: Vec::new() };
    Encode::encode(encode, &mut encoder)?;
    Ok(encoder.buffer)
}

impl Encode for u8 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        encoder.buffer.push(*self);
        Ok(())
    }
}

impl Encode for u16 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        encoder.buffer.extend(u16::to_be_bytes(*self));
        Ok(())
    }
}

impl Encode for u32 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        encoder.buffer.extend(u32::to_be_bytes(*self));
        Ok(())
    }
}

impl Encode for u64 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        encoder.buffer.extend(u64::to_be_bytes(*self));
        Ok(())
    }
}

impl Encode for i8 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        Encode::encode(&(*self as u8), encoder)
    }
}

impl Encode for i16 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        Encode::encode(&(*self as u16), encoder)
    }
}

impl Encode for i32 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        Encode::encode(&(*self as u32), encoder)
    }
}

impl Encode for i64 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        Encode::encode(&(*self as u64), encoder)
    }
}

impl Encode for f32 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        Encode::encode(&f32::to_bits(*self), encoder)?;
        Ok(())
    }
}

impl Encode for f64 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        Encode::encode(&f64::to_bits(*self), encoder)?;
        Ok(())
    }
}

impl Encode for bool {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError> {
        encoder.buffer.push(if *self { 1 } else { 0 });
        Ok(())
    }
}
