use futures::StreamExt;
use log::{error, trace};
use slotmap::{DefaultKey, DenseSlotMap, SlotMap};
use tokio::sync::mpsc::Sender;

use crate::config::CONFIG;

use super::connection::*;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub struct NetworkManager {
    connected: Option<Sender<bool>>,
    listener_thread: Option<JoinHandle<()>>,

    // TODO: Invesigate Lock-free alternatives
    // Locks when a new connection is added/removed
    pub connections: Arc<RwLock<DenseSlotMap<DefaultKey, Mutex<ServerConnection>>>>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            connected: None,
            listener_thread: None,

            connections: Arc::new(RwLock::new(DenseSlotMap::with_key())),
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

            let mut connections = SlotMap::with_capacity(CONFIG.network.max_players);

            let mut df = futures::stream::FuturesUnordered::new();
            let mut cf = futures::stream::FuturesUnordered::new();

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

                                let (connection, mut srv_con, mut disconnect_future) = Connection::new(socket, server_connections.read().await.len()).await;

                                let key = connections.insert(connection);

                                // Disconnect listening
                                df.push(tokio::spawn(async move {
                                    let reason = disconnect_future.recv().await.expect("Go yell at GLS or make a PR if you see this. Error: DF_LAG");
                                    (key, reason)
                                }));
                                cf.push(tokio::spawn(async move {
                                    let opt = srv_con.incoming.recv().await;
                                    if opt.is_some() {
                                        Some(srv_con)
                                    } else {
                                        None
                                    }
                                }));

                                trace!("Connection Accepted. Total: {}", connections.len());
                            }
                            Err(e) => {
                                error!("Error accepting connection: {}", e);
                            }
                        }
                    }
                    Some(Ok((key, reason))) = df.next() => {
                        /*
                            OK so we've got a disconnect request from the connection.
                            We'd naturally remove the connection from the array, but what about server_connections?
                            When a connection is handed over to the server, the server will then handle *all* packets from that connection.
                            Including Disconnects.
                            So the server will itself remove the connection from the server_connections array.
                        */
                        connections.remove(key);
                        if reason.is_empty() {
                            trace!("Connection Closed. Total: {}", connections.len());
                        } else {
                            trace!("Connection Closed. Reason: {}. Total: {}", reason, connections.len());
                        }
                    }
                    Some(Ok(connection)) = cf.next() => {
                        if let Some(connection) = connection {
                            let mut server_connections = server_connections.write().await;
                            server_connections.insert(Mutex::new(connection));
                            trace!("Connection Registered. Total: {}", server_connections.len());
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
