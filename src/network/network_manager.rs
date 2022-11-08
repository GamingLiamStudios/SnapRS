use log::{debug, error};
use slotmap::{DefaultKey, DenseSlotMap};

use crate::config::{BC_CONFIG, CONFIG};

use crate::packets::{clientbound, internal, serverbound, Packets};

use crate::packets::types::{v32, BoundedString, ConnectionState};

use super::{connection::*, raw_packet::RawPacket};
use std::io::Write;
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
            listener.set_nonblocking(true).unwrap(); // rip cpu

            let mut connections = DenseSlotMap::with_capacity(CONFIG.network.max_players);

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

                let online = connections.len();
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
                                    .send(Packets::from(internal::client_packets::SwitchState {
                                        state: connection.state,
                                    }))
                                    .unwrap();
                            }
                            Packets::ServerboundStatusRequest(_) => {
                                #[derive(serde::Serialize)]
                                struct StatusResponse {
                                    version: Version,
                                    players: Players,
                                    description: Chat,

                                    #[serde(skip_serializing_if = "Option::is_none")]
                                    favicon: Option<String>,
                                }

                                #[derive(serde::Serialize)]
                                struct Version {
                                    name: String,
                                    protocol: i32,
                                }

                                #[derive(serde::Serialize)]
                                struct Players {
                                    max: usize,
                                    online: usize,
                                    sample: Vec<Player>,
                                }

                                #[derive(serde::Serialize)]
                                struct Player {
                                    name: String,
                                    id: String,
                                }

                                #[derive(serde::Serialize)]
                                struct Chat {
                                    text: String,
                                }

                                let response = StatusResponse {
                                    version: Version {
                                        name: "1.16.5".to_string(),
                                        protocol: 754,
                                    },
                                    players: Players {
                                        max: CONFIG.network.max_players,
                                        online,
                                        sample: Vec::new(), // TODO
                                    },
                                    description: Chat {
                                        text: CONFIG.server.motd.clone(),
                                    },
                                    favicon: None,
                                };

                                let response = serde_json::to_string(&response).unwrap();
                                debug!("Sending status response: {}", response);

                                connection
                                    .inbound
                                    .send(Packets::from(internal::client_packets::Bounce {
                                        data: Packets::from(
                                            clientbound::status_packets::Response {
                                                json_response: BoundedString::<32767>::from(
                                                    response,
                                                ),
                                            },
                                        ),
                                    }))
                                    .unwrap();
                            }
                            Packets::ServerboundStatusPing(packet) => {
                                connection
                                    .inbound
                                    .send(Packets::from(internal::client_packets::Bounce {
                                        data: Packets::from(clientbound::status_packets::Pong {
                                            payload: packet.payload,
                                        }),
                                    }))
                                    .unwrap();
                                connection
                                    .inbound
                                    .send(Packets::from(internal::client_packets::Disconnect {
                                        reason: BoundedString::<32767>::from("".to_string()),
                                    }))
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                }

                // Send packets to client
                for (key, connection) in connections.iter_mut() {
                    let mut packets: Vec<u8> = Vec::new();

                    while let Ok(packet) = connection.outbound.try_recv() {
                        debug!("Sending packet: {}", packet.get_id());
                        let mut bytes = Vec::new();

                        let id = packet.get_id();

                        match packet {
                            Packets::InternalNetworkDisconnect(packet) => {
                                debug!("Disconnecting client: {}", String::from(packet.reason));
                                remove.push(key);
                            }
                            Packets::InternalNetworkBounce(packet) => {
                                debug!("Bouncing packet to client");
                                bytes.extend(packet.data.get_data());
                            }
                            _ => {
                                bytes.extend(packet.get_data());
                            }
                        }

                        packets.extend(
                            bincode::encode_to_vec(RawPacket { id, data: bytes }, BC_CONFIG)
                                .unwrap(),
                        );
                    }

                    if !packets.is_empty() {
                        debug!("Sending {} bytes to client", packets.len());

                        let mut written = 0;
                        while written < packets.len() {
                            match connection.socket.write(&packets[written..]) {
                                Ok(n) => {
                                    debug!("Wrote {} bytes", n);
                                    written += n;
                                }
                                Err(e) => {
                                    if e.kind() != std::io::ErrorKind::WouldBlock {
                                        error!("Error writing to connection: {}", e);
                                    }
                                    break;
                                }
                            }
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
