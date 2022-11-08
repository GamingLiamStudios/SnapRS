use bincode::{Decode, Encode};

use crate::packets::{types::v32, Packets};

pub struct RawPacket {
    pub id: u8, // Packet ID won't be beyond bounds of u8
    pub data: Vec<u8>,
}

// TODO: will this even work?
trait Compressed {}

/// Serialization
impl bincode::Encode for Packets {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        let bytes = self.get_data();
        let len = bytes.len() as u32;

        <v32 as Encode>::encode(&v32::from(len + 1), encoder)?;
        Encode::encode(&self.get_id(), encoder)?;

        // hate this
        for byte in bytes {
            Encode::encode(&byte, encoder)?;
        }

        Ok(())
    }
}

impl Encode for RawPacket {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        let len = self.data.len() as u32;

        <v32 as Encode>::encode(&v32::from(len + 1), encoder)?;
        Encode::encode(&self.id, encoder)?;

        // hate this
        for byte in &self.data {
            Encode::encode(&byte, encoder)?;
        }

        Ok(())
    }
}

impl Decode for RawPacket {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> core::result::Result<Self, bincode::error::DecodeError> {
        let len: v32 = Decode::decode(decoder)?;
        let id: u8 = Decode::decode(decoder)?;

        let mut data = Vec::with_capacity(u32::from(len) as usize);
        for _ in 0..u32::from(len) as usize {
            data.push(Decode::decode(decoder)?);
        }

        Ok(Self { id, data })
    }
}

/*
impl Encode for Box<dyn Packet + Compressed> {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> core::result::Result<(), bincode::error::EncodeError> {
        todo!("Compressed packets");

        Ok(())
    }
}
*/
