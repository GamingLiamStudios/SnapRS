use crate::{
    encode::{
        self,
        Generate,
    },
    packets::{
        DummyError,
        PacketBuilder,
    },
};

#[derive(Debug)]
pub struct StatusResponse {}

impl PacketBuilder<DummyError> for StatusResponse {
    const PACKET_ID: i32 = 0x00;
}

impl Generate<DummyError> for StatusResponse {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, DummyError> {
        // TODO: Fetch server info
        encode::bounded_string::<255, _>("{\"version\":{\"name\":\"1.20.4\",\"protocol\":765},\"players\":{\"max\":20,\"online\":5},\"description\":{\"text\":\"Hello, world!\"},\"enforcesSecureChat\":false}").generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct StatusPong {
    pub timestamp: i64,
}

impl PacketBuilder<DummyError> for StatusPong {
    const PACKET_ID: i32 = 0x01;
}

impl Generate<DummyError> for StatusPong {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, DummyError> {
        self.timestamp.generate_in_place(buf)
    }
}
