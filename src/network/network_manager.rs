use log::{error, trace};
use slotmap::{DefaultKey, DenseSlotMap};
use tokio::sync::mpsc::Sender;

use crate::config::CONFIG;

use super::connection::*;
use std::sync::{Arc, Mutex};

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

        let server_connections = self.connections.clone();

        self.listener_thread = Some(tokio::task::spawn(async move {
            let listener = TcpListener::bind(format!("127.0.0.1:{}", CONFIG.network.port))
                .await
                .unwrap();

            let mut connections = DenseSlotMap::with_capacity(CONFIG.network.max_players);

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

                                let (connection, srv_con) = Connection::new(socket).await;

                                {
                                    let mut conn = server_connections.lock().unwrap();
                                    (*conn).insert(srv_con);
                                }

                                connections.insert(connection);

                                trace!("Connection Accepted. Total: {}", connections.len());
                            }
                            Err(e) => {
                                error!("Error accepting connection: {}", e);
                            }
                        }
                    }
                }
            }

            // Close all connections
            while let Some((_, connection)) = connections.drain().next() {
                connection.destroy().await;
            }
        }));
    }
}
