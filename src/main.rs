#![feature(generic_const_exprs)]
#![allow(incomplete_features)] // Hate having to do this

use std::{
    error::Error,
    pin::Pin,
};

use packets::{
    ClientConnection,
    StateParser,
    handshake::HandshakingClient,
    login::LoginClient,
    status::StatusClient,
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

mod packets;

pub mod encode;
pub mod parser;
pub mod text;

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
                let mut client = ClientConnection::new(stream);
                let mut next_state = HandshakingClient::new().listen(&mut client).await;
                loop {
                    next_state = match next_state {
                        packets::NextState::None => break,
                        packets::NextState::Status => StatusClient::new().listen(&mut client).await,
                        packets::NextState::Login => LoginClient::new().listen(&mut client).await,
                        packets::NextState::Configure { player: _ }
                        | packets::NextState::Play { player: _ } => todo!(),
                    }
                }
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
