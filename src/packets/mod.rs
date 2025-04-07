use nom::{
    IResult,
    Parser,
    multi::length_value,
};
use smol::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::TcpStream,
};
use tracing::trace;

use crate::{
    encode::{
        self,
        Generate,
    },
    parser::parse_varint,
};

pub mod handshake;
pub mod login;
pub mod status;

#[derive(Debug)]
pub enum PacketError {}

pub trait PacketBuilder: Generate<PacketError> + std::fmt::Debug {
    const PACKET_ID: i32;
}

// TODO: Encryption
#[derive(Clone)]
pub struct ClientConnection {
    stream:          TcpStream,
    pub compression: bool,
}

impl ClientConnection {
    pub const fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            compression: false,
        }
    }

    pub async fn read_raw(
        &mut self,
        buffer: &mut [u8],
    ) -> std::io::Result<usize> {
        self.stream.read(buffer).await
    }

    pub async fn write_raw(
        &mut self,
        buffer: &[u8],
    ) -> std::io::Result<()> {
        self.stream.write_all(buffer).await
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub async fn write_packet<P: PacketBuilder>(
        &mut self,
        packet: P,
    ) -> std::io::Result<()> {
        // TODO: Compression
        trace!(?packet, "Sending Packet");
        let bytes = encode::length_value((encode::write_varint(P::PACKET_ID), packet), |length| {
            encode::write_varint(
                i32::try_from(length).expect("Length of Packet was larger than i32::MAX"),
            )
        })
        .generate()
        .expect("Shouldn't Error");
        self.write_raw(&bytes).await
    }
}

pub trait StateParser: Sized {
    type PacketType<'a>: std::fmt::Debug;
    fn parser(data: &[u8]) -> IResult<&[u8], Self::PacketType<'_>>;

    fn stream(&mut self) -> &mut ClientConnection;

    async fn handle(
        &mut self,
        packet: Self::PacketType<'_>,
    ) -> bool;

    async fn listen(mut self) {
        let mut buffer = vec![0; 2048];
        let mut index = 0;

        buffer.resize(2, 0);
        while let Ok(read) = self.stream().read_raw(&mut buffer[index..]).await {
            if read == 0 {
                break;
            }

            // TODO: Compression
            let next_read;
            match length_value(parse_varint.map(i32::cast_unsigned), Self::parser)
                .parse(&buffer[..index + read])
            {
                Ok((remain, packet)) => {
                    assert!(
                        remain.is_empty(),
                        "Remainder from packet extraction is not empty"
                    );
                    next_read = None;

                    trace!(?packet, "Recv'd packet (size {})", index + read);
                    if self.handle(packet).await {
                        return;
                    }
                },
                Err(nom::Err::Incomplete(needed)) => {
                    next_read = Some(match needed {
                        nom::Needed::Unknown => buffer.len(), // just double the size
                        nom::Needed::Size(n) => n.into(),
                    });

                    index += read;
                },
                error => {
                    // TODO: Safe failure
                    _ = error.expect("Parser Failure");
                    next_read = None;
                },
            }

            if let Some(needed) = next_read {
                trace!("Read {read} bytes, needed {}", read + needed);
                buffer.resize(index + needed, 0);
                continue;
            }

            index = 0;
            buffer.clear();
            buffer.resize(2, 0);
        }
    }
}
