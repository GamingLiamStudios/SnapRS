#![feature(iter_chain)]

use std::{
    error::Error,
    iter::chain,
};

use nom::{
    IResult,
    Parser,
    bits::{
        bits,
        complete::{
            tag,
            take,
        },
    },
    combinator::map,
    multi::many_m_n,
    sequence::{
        pair,
        preceded,
    },
};
use smol::stream::StreamExt;
use tracing::{
    debug,
    info,
};
use tracing_subscriber::{
    Layer,
    filter::Targets,
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

#[derive(thiserror::Error, Debug)]
enum VarintError {
    #[error("Varint is larger than expected.")]
    Oversized,
}

fn parse_varint_bits<const P: u8>(data: &[u8]) -> IResult<&[u8], u8> {
    bits::<_, _, nom::error::Error<(&[u8], usize)>, _, _>(preceded(tag(P, 1usize), take(7usize)))(
        data,
    )
}

#[allow(clippy::cast_possible_wrap)]
fn parse_varint(data: &[u8]) -> IResult<&[u8], i32> {
    map(
        pair(
            many_m_n(1, 4, parse_varint_bits::<1>),
            parse_varint_bits::<0>,
        ),
        |(values, delim)| {
            chain(values.iter(), std::iter::once(&delim))
                .enumerate()
                .fold(0u32, |acc, (i, v)| acc | (u32::from(*v) << (i * 7))) as i32
        },
    )
    .parse(data)
}

async fn run_server() -> Result<(), Box<dyn Error>> {
    let socket = smol::net::TcpListener::bind("127.0.0.1:25565").await?;
    let mut incoming = socket.incoming();

    while let Some(client) = incoming.next().await {
        let client = client?;
        client.set_nodelay(true)?;
        debug!(addr = ?client.peer_addr(), "New client connection");
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let stdout_log = fmt::layer();

    tracing_subscriber::registry()
        .with(
            stdout_log.with_filter(
                Targets::default()
                    .with_target("snap_rs", tracing::Level::DEBUG)
                    .with_default(tracing::Level::INFO),
            ),
        )
        .init();

    smol::block_on(async {
        info!("Hello World!");
        run_server().await
    })
}
