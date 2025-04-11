use std::io::{
    Read,
    Write,
};

use bitflags::bitflags;
use flate2::{
    Compression,
    bufread::ZlibDecoder,
    write::ZlibEncoder,
};
use nom::{
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
    trace,
    warn,
};

use crate::{
    encode::{
        self,
        EncodeError,
        Generate,
    },
    parser::parse_varint,
    text::TextComponent,
};

pub mod configure;
pub mod login;
pub mod play;
pub mod status;

pub const COMPRESSION_MIN_SIZE: i32 = 256;

pub trait PacketError: std::fmt::Debug + From<EncodeError> + From<std::io::Error> {
    fn describe(&self) -> TextComponent;
}

#[derive(Debug)]
pub struct DummyError;

impl From<std::io::Error> for DummyError {
    fn from(_value: std::io::Error) -> Self {
        Self
    }
}

impl From<EncodeError> for DummyError {
    fn from(_value: EncodeError) -> Self {
        Self
    }
}

impl PacketError for DummyError {
    fn describe(&self) -> TextComponent {
        TextComponent::new_text("Unreachable error")
    }
}

pub trait PacketBuilder<E: PacketError>: Generate<E> + std::fmt::Debug {
    const PACKET_ID: i32;
}

// TODO: Encryption
#[derive(Clone)]
pub struct ClientConnection {
    stream: TcpStream,

    buffer_index: usize,
    read_buffer:  Vec<u8>,

    pub compression: bool,
}

impl ClientConnection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buffer_index: 0,
            read_buffer: Vec::with_capacity(2048),
            compression: false,
        }
    }

    pub const fn reset(&mut self) {
        self.buffer_index = 0;
    }

    pub fn take(&self) -> &[u8] {
        &self.read_buffer[..self.buffer_index]
    }

    pub async fn read_to(
        &mut self,
        want: usize,
    ) -> std::io::Result<()> {
        let index = self.buffer_index;
        self.buffer_index = want;
        self.read_buffer.resize(want, 0);
        self.stream
            .read_exact(&mut self.read_buffer[index..want])
            .await
    }

    pub async fn write_raw(
        &mut self,
        buffer: &[u8],
    ) -> std::io::Result<()> {
        self.stream.write_all(buffer).await
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub async fn write_packet<E: PacketError, P: PacketBuilder<E>>(
        &mut self,
        packet: P,
    ) -> Result<(), E> {
        trace!(compression = self.compression, ?packet, "Sending Packet");
        let packet = (encode::write_varint(P::PACKET_ID), packet).generate()?;

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
                    (encode::write_varint::<E>(0), packet.as_slice()).generate_in_place(buf)
                } else {
                    (
                        encode::write_varint(packet.len() as i32),
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
        .generate()?;

        self.write_raw(&bytes)
            .await
            .map_err(std::convert::Into::into)
    }
}

/// Listens for the next packet from a client
///
/// Returns the  packet data in the `buf` parameter
pub async fn recv_packet(
    client: &mut ClientConnection,
    buf: &mut Vec<u8>,
) -> std::io::Result<()> {
    buf.clear();

    let packet_data = {
        client.reset();
        let mut packet_size = 2;
        loop {
            client.read_to(packet_size).await?;

            match length_data(parse_varint.map(i32::cast_unsigned)).parse(client.take()) {
                Ok((_, packet_data)) => {
                    break packet_data;
                },
                Err(nom::Err::Incomplete(nom::Needed::Unknown)) => unreachable!(),
                Err(nom::Err::Incomplete(nom::Needed::Size(needed))) => {
                    packet_size += usize::from(needed);
                },
                Err(nom::Err::Failure(error) | nom::Err::Error(error)) => {
                    warn!(?error, "Parser failure");
                    return Err(std::io::ErrorKind::InvalidData.into());
                },
            }
        }
    };

    if packet_data.len() >= 2usize.pow(21) {
        return Err(std::io::ErrorKind::FileTooLarge.into());
    }

    if client.compression {
        let (payload, inner_size) =
            parse_varint(packet_data).map_err(|_| std::io::ErrorKind::InvalidData)?;

        if inner_size == 0 {
            buf.extend_from_slice(payload);
        } else {
            if inner_size > 2i32.pow(23) {
                return Err(std::io::ErrorKind::FileTooLarge.into());
            }

            // Decompress payload into buf
            let mut decoder = ZlibDecoder::new(payload);
            decoder.read_exact(buf)?;
        }
    } else {
        buf.extend_from_slice(packet_data);
    }

    trace!(bytes = buf.as_slice(), "Recv'd Packet");

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum ChatMode {
    Enabled,
    CommandsOnly,
    Disabled,
}

#[derive(Debug, Clone, Copy)]
pub enum MainHand {
    Left,
    Right,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct EnabledSkinParts: u8 {
        const Cape          = 1 << 0;
        const Jacked        = 1 << 1;
        const LeftSleeve    = 1 << 2;
        const RightSleeve   = 1 << 3;
        const LeftLeg       = 1 << 4;
        const RightLeg      = 1 << 5;
        const Hat           = 1 << 6;
    }
}
