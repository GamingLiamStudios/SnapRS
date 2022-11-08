use log::{debug, error};
use slotmap::{DefaultKey, DenseSlotMap};

use crate::config::{BC_CONFIG, CONFIG};

use crate::packets::*;

use crate::packets::types::{v32, ConnectionState};

use super::{connection::*, raw_packet::RawPacket};
use std::{
    io::Read,
    net::TcpListener,
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread::{self, JoinHandle},
};

pub struct NetworkManager {
    connected: Arc<AtomicBool>,
    listener_thread: Option<JoinHandle<()>>,

    // TODO: Invesigate Lock-free alternatives
    // Locks when a new connection is added/removed
    pub connections: Arc<Mutex<DenseSlotMap<DefaultKey, ServerConnection>>>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            listener_thread: None,

            connections: Arc::new(Mutex::new(DenseSlotMap::with_key())),
        }
    }

    fn destroy(&mut self) {
        self.connected
            .store(false, std::sync::atomic::Ordering::Relaxed);
        if let Some(listener_thread) = self.listener_thread.take() {
            listener_thread.join().unwrap();
        }
    }

    pub fn start(&mut self) {
        self.connected
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let connected = self.connected.clone();
        let server_connections = self.connections.clone();

        self.listener_thread = Some(thread::spawn(move || {
            let listener = TcpListener::bind(format!("127.0.0.1:{}", CONFIG.network.port)).unwrap();
            listener.set_nonblocking(true).unwrap();

            let mut connections = DenseSlotMap::with_capacity(CONFIG.general.max_players);

            while connected.load(std::sync::atomic::Ordering::Relaxed) {
                // Handle all incoming connections(non-blocking)
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            debug!("New connection from {}", stream.peer_addr().unwrap());

                            // Configure TCP Stream
                            stream.set_nonblocking(true).unwrap();
                            //stream.set_nodelay(true).unwrap();

                            // Add connection to list
                            let (net_inbound, con_inbound) = crossbeam_channel::unbounded();
                            let (con_outbound, net_outbound) = crossbeam_channel::unbounded();

                            {
                                let mut conn = server_connections.lock().unwrap();
                                (*conn).insert(ServerConnection::new(con_inbound, con_outbound));
                            }

                            connections.insert(NetworkConnection::new(
                                stream,
                                net_inbound,
                                net_outbound,
                            ));
                            debug!("Connection Accepted. Total: {}", connections.len());
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::WouldBlock {
                                error!("Error accepting connection: {}", e);
                            }
                            break;
                        }
                    }
                }

                // Handle incoming/outgoing data from all connections
                let mut remove = Vec::new();
                let mut bytes = [0; 4096];
                for (key, connection) in connections.iter_mut() {
                    //debug!("Handling connection");

                    let read = match connection.socket.read(&mut bytes) {
                        Ok(0) => {
                            debug!("Connection closed");
                            remove.push(key);
                            0
                        }
                        Ok(n) => {
                            debug!("Read {} bytes", n);
                            n
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::WouldBlock {
                                error!("Error reading from connection: {}", e);
                            }
                            0
                        }
                    };

                    let mut index = 0;
                    while index < read {
                        let (length, size) =
                            bincode::decode_from_slice::<v32, _>(&bytes[index..], BC_CONFIG)
                                .unwrap();
                        let length = u32::from(length) as usize + size;

                        let mut packet_bytes = Vec::with_capacity(length);
                        packet_bytes.extend_from_slice(&bytes[index..]);

                        // Ensure we have enough bytes to read the packet
                        // TODO: Check if we can use packet_bytes.capacity() instead
                        while packet_bytes.len() < length {
                            let mut remain = length - packet_bytes.len();
                            debug!("Packet incomplete. {} bytes remaining", remain);

                            // SAFETY: length <= packet_bytes.capacity()
                            unsafe { packet_bytes.set_len(length) };

                            match connection.socket.read(&mut bytes[read..]) {
                                Ok(n) => {
                                    debug!("Read {} bytes", n);
                                    remain -= n;
                                    // read += n; // Should be fine without

                                    // SAFETY: length <= packet_bytes.capacity()
                                    unsafe { packet_bytes.set_len(length - remain) };
                                }
                                Err(e) => {
                                    if e.kind() != std::io::ErrorKind::WouldBlock {
                                        error!("Error reading from connection: {}", e);
                                    }
                                    break;
                                }
                            }
                        }

                        let (raw, rsize) =
                            bincode::decode_from_slice::<RawPacket, _>(&bytes[index..], BC_CONFIG)
                                .unwrap();
                        index += rsize;

                        let packet = match connection.state {
                            ConnectionState::Handshake => {
                                serverbound::decode_handshaking(u32::from(raw.id) as u8, raw.data)
                            }
                            ConnectionState::Status => {
                                serverbound::decode_status(u32::from(raw.id) as u8, raw.data)
                            }
                            _ => None,
                        };

                        if packet.is_none() {
                            error!("Unknown packet: id {} size {}", u32::from(raw.id), rsize);
                            continue;
                        } else {
                            debug!("Received packet: id {} size {}", u32::from(raw.id), rsize);
                        }

                        let packet = packet.unwrap();

                        match packet {
                            Packets::ServerboundHandshakingHandshake(packet) => {
                                debug!(
                                    "Client connected with protocol version {}",
                                    u32::from(packet.protocol_version)
                                );
                                connection.state = packet.next_state.into();

                                connection
                                    .inbound
                                    .send(Packets::InternalClientSwitchState(Box::new(
                                        internal::client_packets::SwitchState {
                                            state: connection.state,
                                        },
                                    )))
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                }

                // Remove all closed connections
                for key in remove {
                    connections[key]
                        .socket
                        .shutdown(std::net::Shutdown::Both)
                        .unwrap();
                    connections.remove(key);
                    server_connections.lock().unwrap().remove(key);
                }
            }
        }));
    }
}

impl Drop for NetworkManager {
    fn drop(&mut self) {
        self.destroy();
    }
}
