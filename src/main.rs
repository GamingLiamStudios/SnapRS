#![feature(generic_const_exprs, new_range_api, int_roundings)]
#![allow(incomplete_features)] // Hate having to do this

use std::error::Error;

use smol::{
    Executor,
    stream::StreamExt,
};
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

mod client;
mod packets;

pub mod encode;
pub mod parser;
pub mod text;
pub mod world;

pub mod blocks;

async fn run_server() -> Result<(), Box<dyn Error>> {
    let executor = Executor::new();

    executor
        .run(async {
            let socket = smol::net::TcpListener::bind("127.0.0.1:25565").await?;
            let mut incoming = socket.incoming();

            while let Some(client) = incoming.next().await {
                let stream = client?;
                stream.set_nodelay(true)?;
                debug!(addr = ?stream.peer_addr(), "New client connection");

                // Spawn a new task for each client
                executor.spawn(client::spawn(stream)).detach();
            }

            Ok(())
        })
        .await
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
