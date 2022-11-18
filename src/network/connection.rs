use std::sync::Arc;

use log::{debug, error, trace, warn};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use crate::packets::{self, serial};
use crate::{config::CONFIG, packets::Packets};

use crate::packets::types::{v32, BoundedString, ConnectionState};

use tokio::sync::{
    broadcast,
    mpsc::{Receiver, Sender},
};

const PROTOCOL_VERSION: u32 = 754;

pub struct ServerConnection {
    pub incoming: Receiver<Packets>,
    pub outgoing: Sender<Packets>,
}

pub(crate) struct Connection {
    //pub(crate) outgoing: Sender<Packets>,
    connected: broadcast::Sender<bool>,

    writer: tokio::task::JoinHandle<()>,
    reader: tokio::task::JoinHandle<()>,
}

impl Connection {
    pub(crate) async fn new(
        socket: TcpStream,
    ) -> (Self, ServerConnection, broadcast::Receiver<bool>) {
        let (inbound, incoming) = tokio::sync::mpsc::channel(32);
        let (outgoing, mut outbound) = tokio::sync::mpsc::channel::<Packets>(32);

        // How did I get this number? Spamming 'Refresh' in the server list until I didn't get a LAGGED error
        let (ctx, crx) = broadcast::channel(5);

        // Probably isn't needed but just in case
        // NOTE: Race Condition prevention
        let crx1 = crx.resubscribe();
        let crx2 = crx.resubscribe();

        let (reader, writer) = socket.into_split();

        let mut compressed = Arc::new(RwLock::new(false));

        // TODO: Figure out what to do with recv/send errors

        // Write to Client
        let ctxc = ctx.clone();
        let mut cc = compressed.clone();
        let writer = tokio::spawn(async move {
            let mut crx = crx1;
            let ctx = ctxc;
            let mut compressed = cc;
            loop {
                tokio::select! {
                    _ = crx.recv() => {
                        break;
                    }
                    packet = outbound.recv() => {
                        if let Some(packet) = packet {
                            trace!("Sending packet: {}", packet.get_id());
                            let mut bytes = Vec::new();

                            let id = packet.get_id();

                            match packet {
                                Packets::InternalNetworkDisconnect(packet) => {
                                    debug!("Disconnecting client: {}", String::from(packet.reason));
                                    ctx.send(true).unwrap();
                                }
                                _ => {
                                    bytes.extend(packet.get_data());
                                }
                            }

                            let len = (bytes.len() + 1) as u32;
                            let mut data = Vec::with_capacity(bytes.len() + 1 + v32::byte_size(len));
                            data.extend(serial::encode_to_vec(&v32::from(len)).unwrap());
                            data.push(id);
                            data.extend(bytes);

                            trace!("Sending {} bytes to client", data.len());

                            let mut written = 0;
                            while written < data.len() {
                                writer.writable().await.unwrap();
                                match writer.try_write(&data[written..]) {
                                    Ok(n) => {
                                        trace!("Wrote {} bytes", n);
                                        written += n;
                                    }
                                    Err(e) => {
                                        if e.kind() != std::io::ErrorKind::WouldBlock {
                                            error!("Error reading from connection: {}", e);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Shouldn't be needed
            //ctx.send(true).unwrap();
        });

        // Read from Client
        let ctxc = ctx.clone();
        let outgoing_clone = outgoing.clone();
        let reader = tokio::spawn(async move {
            let mut state = ConnectionState::Handshake;

            let mut crx = crx2;
            let ctx = ctxc;

            // Buffer for reading data from the client
            let mut buffer = vec![0; CONFIG.network.advanced.buffer_size];

            'outer: loop {
                tokio::select! {
                    _ = crx.recv() => {
                        break 'outer;
                    }
                    _ = reader.readable() => {
                    }
                }

                let read = match reader.try_read(&mut buffer) {
                    Ok(0) => {
                        //trace!("Connection closed");
                        ctx.send(true).unwrap();
                        0
                    }
                    Ok(n) => {
                        trace!("Read {} bytes", n);
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
                    let (length, lsize) = v32::read_from_slice(&buffer[index..]);
                    let length = u32::from(length) as usize + lsize;

                    let mut packet_bytes = Vec::with_capacity(length);

                    // Ensure we index within buffer bounds
                    // SAFETY: chunks(length) will never panic as length is always guarenteed to be at least 2
                    let b = buffer[index..].chunks(length).next().unwrap();
                    packet_bytes.extend_from_slice(b);
                    index += b.len();

                    // Ensure we have read the entire packet
                    // TODO: Check if we can use packet_bytes.capacity() instead
                    let mut remain = length - packet_bytes.len();

                    // Same reason as above
                    // SAFETY: length <= packet_bytes.capacity()
                    packet_bytes.resize(length, 0);

                    while remain > 0 {
                        trace!("Packet incomplete. {} bytes remaining", remain);

                        reader.readable().await.unwrap();
                        match reader.try_read(&mut packet_bytes[(length - remain)..]) {
                            Ok(0) => {
                                //trace!("Connection closed");
                                trace!("Connection close while reading packet");
                                ctx.send(true).unwrap();
                                break 'outer;
                            }
                            Ok(n) => {
                                trace!("Read {} bytes", n);
                                remain -= n;
                            }
                            Err(e) => {
                                if e.kind() != std::io::ErrorKind::WouldBlock {
                                    error!("Error reading from connection: {}", e);
                                }
                            }
                        }
                    }

                    // TODO: Compression
                    // TODO: Encryption
                    let id = packet_bytes[lsize];
                    let data = &packet_bytes[lsize + 1..];
                    //index += packet_bytes.len();

                    let packet = match state {
                        ConnectionState::Handshake => packets::serverbound::decode_handshaking(
                            u32::from(id) as u8,
                            data.to_vec(),
                        ),
                        ConnectionState::Status => {
                            packets::serverbound::decode_status(u32::from(id) as u8, data.to_vec())
                        }
                        ConnectionState::Login => {
                            packets::serverbound::decode_login(u32::from(id) as u8, data.to_vec())
                        }
                        _ => None,
                    };

                    if packet.is_none() {
                        error!(
                            "Unknown packet: id {} size {}",
                            u32::from(id),
                            packet_bytes.len()
                        );
                        continue;
                    } else {
                        debug!(
                            "Received packet: id {} size {}",
                            u32::from(id),
                            packet_bytes.len()
                        );
                    }

                    let packet = packet.unwrap();

                    if process_packet(&packet, &mut state, &outgoing_clone, &ctx).await {
                        inbound.send(packet).await.unwrap();
                    }
                }
            }
        });

        (
            Self {
                //outgoing,
                connected: ctx,
                writer,
                reader,
            },
            ServerConnection { incoming, outgoing },
            crx,
        )
    }

    pub async fn destroy(self) {
        if self.connected.receiver_count() == 0
            || self.writer.is_finished()
            || self.reader.is_finished()
        {
            return;
        }

        self.connected.send(true).unwrap();

        self.writer.await.unwrap();
        self.reader.await.unwrap();
    }
}

// Returns true if the packet should be sent to the server
async fn process_packet(
    packet: &Packets,
    state: &mut ConnectionState,
    outgoing: &Sender<Packets>,
    close_sender: &broadcast::Sender<bool>,
) -> bool {
    match packet {
        Packets::ServerboundHandshakingHandshake(packet) => {
            let ver = u32::from(packet.protocol_version);
            debug!("Client connected with protocol version {}", ver);
            if ver > PROTOCOL_VERSION && packet.next_state != 1 {
                warn!(
                    "Client attempted connection with Unsupported Protocol Version {}",
                    u32::from(packet.protocol_version)
                );
                close_sender.send(true).unwrap();
            }

            *state = packet.next_state.into();
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
                    online: 0,          // TODO: Get online players from Server
                    sample: Vec::new(), // TODO
                },
                description: Chat {
                    text: CONFIG.server.motd.clone(),
                },
                favicon: None,
            };

            let response = serde_json::to_string(&response).unwrap();

            outgoing
                .send(Packets::from(
                    packets::clientbound::status_packets::Response {
                        json_response: BoundedString::<32767>::from(response),
                    },
                ))
                .await
                .unwrap();
        }
        Packets::ServerboundStatusPing(packet) => {
            outgoing
                .send(Packets::from(packets::clientbound::status_packets::Pong {
                    payload: packet.payload,
                }))
                .await
                .unwrap();
        }
        Packets::ServerboundLoginLoginStart(packet) => {
            debug!(
                "Client with username '{}' attempting connection",
                packet.name
            );
        }
        _ => {
            return true;
        }
    }
    false
}
