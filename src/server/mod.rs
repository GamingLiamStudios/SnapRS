use std::sync::{atomic::AtomicBool, Arc};

use log::debug;

use crate::network::NetworkManager;

pub struct Server {
    network_manager: NetworkManager,

    pub running: Arc<AtomicBool>,
}

impl Server {
    pub fn new(running: Arc<AtomicBool>) -> Self {
        Self {
            network_manager: NetworkManager::new(),
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

    async fn process_connections(&self) {
        let mut connections = self.network_manager.connections.lock().await;
        for (_, connection) in &mut *connections {
            // Read all incoming packets
            while let Ok(packet) = connection.incoming.try_recv() {
                match packet {
                    _ => {}
                }
            }
        }
    }
}

pub async fn start(running: Arc<AtomicBool>) {
    let mut server = Server::new(running);
    server.start().await;
}
