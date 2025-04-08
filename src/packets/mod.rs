use std::io::{
    Read,
    Write,
};

use flate2::{
    Compression,
    bufread::ZlibDecoder,
    write::ZlibEncoder,
};
use nom::{
    IResult,
    Parser,
    multi::length_data,
};
use smol::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::TcpStream,
};
use tracing::{
    info,
    trace,
};

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

pub const COMPRESSION_MIN_SIZE: i32 = 256;

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
        trace!(compression = self.compression, ?packet, "Sending Packet");
        let packet = (encode::write_varint(P::PACKET_ID), packet)
            .generate()
            .expect("Shouldn't Error");

        let inner = |buf: &mut Vec<u8>| {
            if self.compression {
                // Try to compress bytes
                let mut deflate = ZlibEncoder::new(Vec::new(), Compression::best());
                deflate
                    .write_all(&packet)
                    .expect("Failed to compress packet");
                let compressed_bytes = deflate.flush_finish().expect("Failed to compress packet");
                let inner_size = compressed_bytes.len();

                if packet.len() < COMPRESSION_MIN_SIZE.cast_unsigned() as usize
                    || inner_size > packet.len()
                {
                    (encode::write_varint::<PacketError>(0), packet.as_slice())
                        .generate_in_place(buf)
                } else {
                    (
                        encode::write_varint::<PacketError>(packet.len() as i32),
                        compressed_bytes.as_slice(),
                    )
                        .generate_in_place(buf)
                }
            } else {
                packet.as_slice().generate_in_place(buf)
            }
        };

        let bytes = encode::length_value(inner, |length| {
            encode::write_varint(
                i32::try_from(length).expect("Length of Packet was larger than i32::MAX"),
            )
        })
        .generate()
        .expect("Shouldn't error");

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

            let next_read;
            match length_data(parse_varint.map(i32::cast_unsigned)).parse(&buffer[..index + read]) {
                Ok((remain, data)) => {
                    assert!(
                        remain.is_empty(),
                        "Remainder from packet extraction is not empty"
                    );
                    next_read = None;

                    if self.stream().compression {
                        let (data, inner_size) =
                            parse_varint(data).expect("Failed to parse packet");
                        if inner_size > 0 {
                            // Decompress
                            let mut bytes = vec![0u8; inner_size.cast_unsigned() as usize];
                            let mut decoder = ZlibDecoder::new(data);
                            decoder
                                .read_exact(&mut bytes)
                                .expect("Failed to decompress packet");

                            let (_data, packet) =
                                Self::parser(&bytes).expect("Failed to parse packet");
                            trace!(?packet, "Recv'd packet (size {})", index + read);
                            if self.handle(packet).await {
                                return;
                            }
                        } else {
                            let (_data, packet) =
                                Self::parser(data).expect("Failed to parse packet");
                            trace!(?packet, "Recv'd packet (size {})", index + read);
                            if self.handle(packet).await {
                                return;
                            }
                        }
                    } else {
                        let (_data, packet) = Self::parser(data).expect("Failed to parse packet");
                        trace!(?packet, "Recv'd packet (size {})", index + read);
                        if self.handle(packet).await {
                            return;
                        }
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
