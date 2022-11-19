use futures::StreamExt;
use log::{error, trace};
use slotmap::{DefaultKey, DenseSlotMap, SlotMap};
use tokio::sync::mpsc::Sender;

use crate::config::CONFIG;

use super::connection::*;
use std::sync::Arc;
use tokio::sync::Mutex;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub struct NetworkManager {
    connected: Option<Sender<bool>>,
    listener_thread: Option<JoinHandle<()>>,

    // TODO: Invesigate Lock-free alternatives
    // Locks when a new connection is added/removed
    pub connections: Arc<Mutex<DenseSlotMap<DefaultKey, ServerConnection>>>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            connected: None,
            listener_thread: None,

            connections: Arc::new(Mutex::new(DenseSlotMap::with_key())),
        }
    }

    pub async fn stop(&mut self) {
        if let Some(connected) = self.connected.take() {
            connected.send(true).await.unwrap();
        }
        if let Some(listener_thread) = self.listener_thread.take() {
            listener_thread.await.unwrap();
        }
    }

    pub async fn start(&mut self) {
        let (ctx, mut crx) = tokio::sync::mpsc::channel(1);
        self.connected = Some(ctx);

        let _server_connections = self.connections.clone();

        self.listener_thread = Some(tokio::task::spawn(async move {
            let listener = TcpListener::bind(format!("127.0.0.1:{}", CONFIG.network.port))
                .await
                .unwrap();

            let mut connections = SlotMap::with_capacity(CONFIG.network.max_players);

            let mut df = futures::stream::FuturesUnordered::new();

            // Handle all incoming connections
            loop {
                tokio::select! {
                    _ = crx.recv() => {
                        break;
                    }
                    incoming = listener.accept() => {
                        match incoming {
                            Ok((socket, addr)) => {
                                trace!("New connection from {}", addr);

                                // Configure TCP Stream
                                socket.set_nodelay(true).unwrap();

                                let (connection, _srv_con, mut disconnect_future) = Connection::new(socket).await;

                                let key = connections.insert(connection);

                                // Disconnect listening
                                df.push(tokio::spawn(async move {
                                    let (_, reason) = disconnect_future.recv().await.expect("Go yell at GLS or make a PR if you see this. Error: DF_LAG");
                                    (key, reason)
                                }));

                                trace!("Connection Accepted. Total: {}", connections.len());
                            }
                            Err(e) => {
                                error!("Error accepting connection: {}", e);
                            }
                        }
                    }
                    Some(Ok((key, reason))) = df.next() => {
                        connections.remove(key);
                        if reason.is_empty() {
                            trace!("Connection Closed. Total: {}", connections.len());
                        } else {
                            trace!("Connection Closed. Reason: {}. Total: {}", reason, connections.len());
                        }
                    }
                }
            }

            // TODO: Push connection to server_connections once it is fully connected

            // Close all connections
            while let Some((_, connection)) = connections.drain().next() {
                connection.destroy().await;
            }
        }));
    }
}
