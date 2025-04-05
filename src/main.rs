#![feature(iter_chain)]

use std::{
    error::Error,
    iter::chain,
    pin::Pin,
};

use nom::{
    IResult,
    Parser,
    bits::{
        bits,
        streaming::{
            tag,
            take,
        },
    },
    combinator::{
        map,
        verify,
    },
    multi::{
        length_data,
        length_value,
        many_m_n,
    },
    number::streaming::{
        be_i64,
        be_u16,
    },
    sequence::{
        pair,
        preceded,
    },
};
use smol::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::TcpStream,
    stream::StreamExt,
};
use tracing::{
    debug,
    info,
    trace,
    warn,
};
use tracing_subscriber::{
    Layer,
    filter::Targets,
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

fn parse_varbits<const P: u8>(data: &[u8]) -> IResult<&[u8], u8> {
    bits::<_, _, nom::error::Error<(&[u8], usize)>, _, _>(preceded(tag(P, 1usize), take(7usize)))(
        data,
    )
}

// TODO: Use more descriptive/appropriate error
#[allow(clippy::cast_possible_wrap)]
fn parse_varint(data: &[u8]) -> IResult<&[u8], i32> {
    map(
        pair(many_m_n(0, 4, parse_varbits::<1>), parse_varbits::<0>),
        |(values, delim)| {
            chain(values.iter(), std::iter::once(&delim))
                .enumerate()
                .fold(0u32, |acc, (i, v)| acc | (u32::from(*v) << (i * 7))) as i32
        },
    )
    .parse(data)
}

#[allow(clippy::cast_possible_wrap)]
fn parse_varlong(data: &[u8]) -> IResult<&[u8], i64> {
    map(
        pair(many_m_n(0, 9, parse_varbits::<1>), parse_varbits::<0>),
        |(values, delim)| {
            chain(values.iter(), std::iter::once(&delim))
                .enumerate()
                .fold(0u64, |acc, (i, v)| acc | (u64::from(*v) << (i * 7))) as i64
        },
    )
    .parse(data)
}

fn parse_string<const MAX: i32>(data: &[u8]) -> IResult<&[u8], &str> {
    assert!(MAX <= 32767, "Invalid Maximum Size");
    length_data(verify(parse_varint, |v| *v <= (MAX * 3)).map(i32::cast_unsigned))
        .map_res(|v| str::from_utf8(v))
        .parse(data)
}

fn write_varint(
    value: i32,
    buffer: &mut Vec<u8>,
) {
    let mut value = value.cast_unsigned();
    for _ in 0..5 {
        let nibble = (value & 0x7f) as u8;
        value >>= 7;

        if value != 0 {
            buffer.push(0x80 | nibble);
        } else {
            buffer.push(nibble);
            return;
        }
    }

    warn!("Wrote more than 5 bytes with remaining data");
}

fn write_varlong(
    value: i64,
    buffer: &mut Vec<u8>,
) {
    let mut value = value.cast_unsigned();
    for _ in 0..10 {
        let nibble = (value & 0x7f) as u8;
        value >>= 7;

        if value != 0 {
            buffer.push(0x80 | nibble);
        } else {
            buffer.push(nibble);
            return;
        }
    }

    warn!("Wrote more than 5 bytes with remaining data");
}

const fn varint_len(value: i32) -> usize {
    match value {
        ..0 => 5,
        0 => 1,
        v => v.ilog2().div_ceil(7) as usize,
    }
}

const fn varlong_len(value: i32) -> usize {
    match value {
        ..0 => 10,
        0 => 1,
        v => v.ilog2().div_ceil(7) as usize,
    }
}

#[derive(Debug)]
enum NextState {
    Status,
    Login,
    Transfer,
}

#[derive(Debug)]
#[allow(dead_code)]
enum HandshakePacket<'a> {
    Intention {
        protocol_version: i32,
        server_address:   &'a str,
        server_port:      u16,
        next_state:       NextState,
    },
    LegacyPing {
        // TODO
    },
}

trait PacketBuilder {
    // TODO: Allow faliable packet construction (invalid state)
    fn build(&self) -> (i32, Box<[u8]>);
}

struct StatusResponse {
    // TODO: Store info about server
}

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
impl PacketBuilder for StatusResponse {
    fn build(&self) -> (i32, Box<[u8]>) {
        let json = "{\"version\":{\"name\":\"1.20.4\",\"protocol\":765},\"players\":{\"max\":20,\"online\":5},\"description\":{\"text\":\"Hello, world!\"},\"enforcesSecureChat\":false}";
        let bytes = json.as_bytes();

        let mut buf = Vec::with_capacity(varint_len(bytes.len() as i32) + bytes.len());
        write_varint(bytes.len() as i32, &mut buf);
        buf.extend_from_slice(bytes);

        (0x00, buf.into_boxed_slice())
    }
}

struct StatusPong {
    timestamp: i64,
}

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
impl PacketBuilder for StatusPong {
    fn build(&self) -> (i32, Box<[u8]>) {
        let mut buf = Vec::with_capacity(varint_len(size_of::<i64>() as i32) + size_of::<i64>());
        write_varint(size_of::<i64>() as i32, &mut buf);
        buf.extend_from_slice(&self.timestamp.to_be_bytes());

        (0x01, buf.into_boxed_slice())
    }
}

// TODO: Encryption
#[derive(Clone)]
struct ClientConnection {
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
    pub async fn write_packet(
        &mut self,
        packet: impl PacketBuilder,
    ) -> std::io::Result<()> {
        let (id, data) = packet.build();

        // TODO: Compression
        let packet_length = data.len() + varint_len(id);
        let mut buf = Vec::with_capacity(varint_len(packet_length as i32) + packet_length);
        write_varint(packet_length as i32, &mut buf);
        write_varint(id, &mut buf);
        buf.extend_from_slice(&data);

        self.stream.write_all(&buf).await
    }
}

trait StateParser: Sized {
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
            debug!("Read {} bytes", read);
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

                    trace!(?packet);
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
                    _ = error.expect("Parser Failure");
                    next_read = None;
                },
            }

            if let Some(needed) = next_read {
                debug!("Packet not large enough, needed {needed} more bytes",);
                buffer.resize(index + needed, 0);
                continue;
            }

            index = 0;
            buffer.clear();
            buffer.resize(2, 0);
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum StatusPacket {
    Request,
    Ping(i64),
}

struct StatusClient {
    stream: ClientConnection,
}

impl StatusClient {
    pub const fn new(stream: ClientConnection) -> Self {
        Self { stream }
    }
}

impl StateParser for StatusClient {
    type PacketType<'a> = StatusPacket;

    fn stream(&mut self) -> &mut ClientConnection {
        &mut self.stream
    }

    fn parser(data: &[u8]) -> IResult<&[u8], Self::PacketType<'_>> {
        let (data, packet_id) = parse_varint(data)?;
        match packet_id {
            0x00 => Ok((data, StatusPacket::Request)),
            0x01 => {
                let (data, timestamp) = be_i64(data)?;
                Ok((data, StatusPacket::Ping(timestamp)))
            },
            _ => unimplemented!("No other packets exist for Status"),
        }
    }

    async fn handle(
        &mut self,
        packet: Self::PacketType<'_>,
    ) -> bool {
        match packet {
            StatusPacket::Request => {
                // Send Response packet
                _ = self.stream.write_packet(StatusResponse {}).await;
                false
            },
            StatusPacket::Ping(timestamp) => {
                // Send pong packet
                _ = self.stream.write_packet(StatusPong { timestamp }).await;
                true
            },
        }
    }
}

struct HandshakingClient {
    stream: ClientConnection,
}

impl HandshakingClient {
    pub const fn new(stream: ClientConnection) -> Self {
        Self { stream }
    }
}

impl StateParser for HandshakingClient {
    type PacketType<'a> = HandshakePacket<'a>;

    fn stream(&mut self) -> &mut ClientConnection {
        &mut self.stream
    }

    fn parser(data: &[u8]) -> IResult<&[u8], Self::PacketType<'_>> {
        let (data, packet_id) = parse_varint(data)?;
        match packet_id {
            0x00 => {
                let (data, (protocol_version, server_address, server_port, next)) =
                    (parse_varint, parse_string::<256>, be_u16, parse_varint).parse(data)?;
                Ok((data, HandshakePacket::Intention {
                    protocol_version,
                    server_address,
                    server_port,
                    next_state: match next {
                        1 => NextState::Status,
                        2 => NextState::Login,
                        3 => NextState::Transfer,
                        _ => unreachable!("Further states do not exist"),
                    },
                }))
            },
            _ => unimplemented!("No other states exist for Packet Handshake"),
        }
    }

    async fn handle(
        &mut self,
        packet: Self::PacketType<'_>,
    ) -> bool {
        match packet {
            HandshakePacket::Intention {
                protocol_version,
                server_address: _,
                server_port: _,
                next_state,
            } => {
                if protocol_version != 765 {
                    debug!(
                        protocol_version,
                        "Attempted connection from incompatible client"
                    );
                    return false; // Quit the client
                }

                match next_state {
                    NextState::Status => {
                        // Enter status flow
                        info!("Entering Status netflow");
                        StatusClient::new(self.stream.clone()).listen().await;
                    },
                    NextState::Login => {
                        // Enter login flow
                        info!("Entering Login netflow");
                        unimplemented!();
                    },
                    NextState::Transfer => {
                        unimplemented!("i honestly have no idea what this is");
                    },
                }
            },
            HandshakePacket::LegacyPing {} => {
                unimplemented!("Legacy ping unimplemented");
            },
        }

        true
    }
}

async fn run_server() -> Result<(), Box<dyn Error>> {
    let socket = smol::net::TcpListener::bind("127.0.0.1:25565").await?;
    let mut incoming = socket.incoming();

    while let Some(client) = incoming.next().await {
        let stream = client?;
        stream.set_nodelay(true)?;
        debug!(addr = ?stream.peer_addr(), "New client connection");

        // Spawn a new task for each client
        smol::spawn(unsafe {
            std::mem::transmute::<
                Pin<Box<dyn Future<Output = ()>>>,
                Pin<Box<dyn Future<Output = ()> + 'static + Send>>,
            >(Box::pin(async move {
                HandshakingClient::new(ClientConnection::new(stream))
                    .listen()
                    .await;
                info!("Connection closed");
            }))
        })
        .await;
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let stdout_log = fmt::layer();

    tracing_subscriber::registry()
        .with(
            stdout_log.with_filter(
                Targets::default()
                    .with_target("snap_rs", tracing::Level::TRACE)
                    .with_default(tracing::Level::INFO),
            ),
        )
        .init();

    smol::block_on(async {
        info!("Hello World!");
        run_server().await
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_parse() {
        assert_eq!(parse_varint(&[0x00]).map(|(_, v)| v), Ok(0));
        assert_eq!(parse_varint(&[0x01]).map(|(_, v)| v), Ok(1));
        assert_eq!(parse_varint(&[0x02]).map(|(_, v)| v), Ok(2));
        assert_eq!(parse_varint(&[0x7f]).map(|(_, v)| v), Ok(127));
        assert_eq!(parse_varint(&[0x80, 0x01]).map(|(_, v)| v), Ok(128));
        assert_eq!(parse_varint(&[0xff, 0x01]).map(|(_, v)| v), Ok(255));
        assert_eq!(parse_varint(&[0xdd, 0xc7, 0x01]).map(|(_, v)| v), Ok(25565));
        assert_eq!(
            parse_varint(&[0xff, 0xff, 0x7f]).map(|(_, v)| v),
            Ok(2_097_151)
        );
        assert_eq!(
            parse_varint(&[0xff, 0xff, 0xff, 0xff, 0x07]).map(|(_, v)| v),
            Ok(2_147_483_647)
        );
        assert_eq!(
            parse_varint(&[0xff, 0xff, 0xff, 0xff, 0x0f]).map(|(_, v)| v),
            Ok(-1)
        );
        assert_eq!(
            parse_varint(&[0x80, 0x80, 0x80, 0x80, 0x08]).map(|(_, v)| v),
            Ok(-2_147_483_648)
        );

        assert_eq!(
            parse_varint(&[0xff, 0xff, 0xff, 0xff, 0xff]),
            Err(nom::Err::Error(nom::error::Error {
                input: [255u8].as_slice(),
                code:  nom::error::ErrorKind::TagBits,
            }))
        );
    }

    #[test]
    fn test_varlong_parse() {
        assert_eq!(parse_varlong(&[0x00]).map(|(_, v)| v), Ok(0));
        assert_eq!(parse_varlong(&[0x01]).map(|(_, v)| v), Ok(1));
        assert_eq!(parse_varlong(&[0x02]).map(|(_, v)| v), Ok(2));
        assert_eq!(parse_varlong(&[0x7f]).map(|(_, v)| v), Ok(127));

        assert_eq!(parse_varlong(&[0x80, 0x01]).map(|(_, v)| v), Ok(128));
        assert_eq!(parse_varlong(&[0xff, 0x01]).map(|(_, v)| v), Ok(255));

        assert_eq!(
            parse_varlong(&[0xff, 0xff, 0x7f]).map(|(_, v)| v),
            Ok(2_097_151)
        );
        assert_eq!(
            parse_varlong(&[0xff, 0xff, 0xff, 0xff, 0x07]).map(|(_, v)| v),
            Ok(2_147_483_647)
        );
        assert_eq!(
            parse_varlong(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]).map(|(_, v)| v),
            Ok(9_223_372_036_854_775_807)
        );

        assert_eq!(
            parse_varlong(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01])
                .map(|(_, v)| v),
            Ok(-1)
        );
        assert_eq!(
            parse_varlong(&[0x80, 0x80, 0x80, 0x80, 0xf8, 0xff, 0xff, 0xff, 0xff, 0x01])
                .map(|(_, v)| v),
            Ok(-2_147_483_648)
        );
        assert_eq!(
            parse_varlong(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01])
                .map(|(_, v)| v),
            Ok(-9_223_372_036_854_775_808)
        );

        assert_eq!(
            parse_varlong(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            Err(nom::Err::Error(nom::error::Error {
                input: [255u8].as_slice(),
                code:  nom::error::ErrorKind::TagBits,
            }))
        );
    }
}
