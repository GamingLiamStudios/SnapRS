#![feature(generic_const_exprs)]
#![allow(
    clippy::ref_option_ref,
    clippy::used_underscore_binding,
    incomplete_features
)] // Hate having to do this

use std::{
    error::Error,
    pin::Pin,
};

use packets::{
    ClientConnection,
    StateParser,
    handshake::HandshakingClient,
};
use smol::stream::StreamExt;
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

pub(crate) mod encode;
mod packets;
pub(crate) mod parser;

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
