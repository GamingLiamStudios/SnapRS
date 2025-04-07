use crate::{
    encode::{
        self,
        Generate,
    },
    packets::{
        PacketBuilder,
        PacketError,
    },
};

#[derive(Debug)]
pub struct StatusResponse {
    // TODO: Store info about server
}

impl PacketBuilder for StatusResponse {
    const PACKET_ID: i32 = 0x00;
}

impl Generate<PacketError> for StatusResponse {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PacketError> {
        encode::bounded_string::<255, _>("{\"version\":{\"name\":\"1.20.4\",\"protocol\":765},\"players\":{\"max\":20,\"online\":5},\"description\":{\"text\":\"Hello, world!\"},\"enforcesSecureChat\":false}").generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct StatusPong {
    pub timestamp: i64,
}

impl PacketBuilder for StatusPong {
    const PACKET_ID: i32 = 0x01;
}

impl Generate<PacketError> for StatusPong {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PacketError> {
        self.timestamp.generate_in_place(buf)
    }
}
