use log::debug;

use crate::{
    network::NetworkManager,
    packets::{internal, Packets},
};

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
        let mut connections = self.network_manager.connections.lock().unwrap();
        for (_, connection) in &mut *connections {
            // Read all incoming packets
            while let Ok(packet) = connection.inbound.try_recv() {
                match packet {
                    Packets::InternalClientSwitchState(packet) => {
                        connection.state = packet.state;
                        debug!("Client switched state to {}", u8::from(packet.state));
                    }
                    Packets::InternalClientBounce(packet) => {
                        connection
                            .outbound
                            .send(Packets::from(internal::network_packets::Bounce {
                                data: packet.data,
                            }))
                            .unwrap();
                    }
                    Packets::InternalClientDisconnect(packet) => {
                        connection
                            .outbound
                            .send(Packets::from(internal::network_packets::Disconnect {
                                reason: packet.reason,
                            }))
                            .unwrap();
                    }
                    _ => {}
                }
            }
        }
    }
}
