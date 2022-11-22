mod player;

use std::sync::{atomic::AtomicBool, Arc};

use log::{debug, trace};
use player::Player;
use slotmap::{DefaultKey, DenseSlotMap};

use crate::{
    network::{connection::ServerConnection, NetworkManager},
    packets::Packets,
};

pub struct Server {
    network_manager: NetworkManager,
    players: DenseSlotMap<DefaultKey, Player>,

    pub running: Arc<AtomicBool>,
}

impl Server {
    pub fn new(running: Arc<AtomicBool>) -> Self {
        Self {
            network_manager: NetworkManager::new(),
            players: DenseSlotMap::new(),
            running,
        }
    }

    pub async fn start(&mut self) {
        self.network_manager.start().await;

        self.running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        while self.running.load(std::sync::atomic::Ordering::Relaxed) {
            self.process_connections().await;
        }
        debug!("Server stopped");

        self.network_manager.stop().await;
    }

    async fn process_connections(&mut self) {
        let mut disconnections = Vec::new();

        let connections = self.network_manager.connections.clone();
        let connections = connections.write().await;
        for (key, connection) in connections.iter() {
            loop {
                trace!("Processing connection {:?}", key);
                let mut connection = connection.lock().await;
                let recv = connection.incoming.try_recv();
                match recv {
                    Ok(packet) => {
                        self.process_packet(packet, &connection, key).await;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        break;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        disconnections.push(key);
                        break;
                    }
                }
            }
        }

        if !disconnections.is_empty() {
            drop(connections);
            let mut connections = self.network_manager.connections.write().await;
            for key in disconnections {
                connections.remove(key);
            }
        }
    }

    async fn process_packet(
        &mut self,
        packet: Packets,
        connection: &ServerConnection,
        key: DefaultKey,
    ) {
        match packet {
            Packets::InternalServerInitalize(packet) => {
                self.players.insert(Player {
                    key,
                    username: packet.username,
                    uuid: packet.uuid,
                });
            }
            _ => {}
        }
    }
}

pub async fn start(running: Arc<AtomicBool>) {
    let mut server = Server::new(running);
    server.start().await;
}
