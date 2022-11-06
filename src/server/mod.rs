use crate::network::NetworkManager;

pub struct Server {
    network_manager: NetworkManager,
}

impl Server {
    pub fn new() -> Self {
        Self {
            network_manager: NetworkManager::new(),
        }
    }

    pub fn start(&mut self) {
        self.network_manager.start();

        loop {
            self.process_connections();
        }
    }

    fn process_connections(&self) {
        let connections = self.network_manager.connections.lock().unwrap();
        for (id, connection) in &*connections {}
    }
}
