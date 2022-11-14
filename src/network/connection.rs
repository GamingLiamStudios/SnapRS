use log::{debug, error, trace};
use tokio::net::TcpStream;

use crate::config::BC_CONFIG;
use crate::network::raw_packet::RawPacket;
use crate::packets;
use crate::{config::CONFIG, packets::Packets};

use crate::packets::types::{v32, BoundedString, ConnectionState};

use tokio::sync::{
    broadcast,
    mpsc::{Receiver, Sender},
};

pub struct ServerConnection {
    pub incoming: Receiver<Packets>,
    pub outgoing: Sender<Packets>,
}

pub(crate) struct Connection {
    //pub(crate) outgoing: Sender<Packets>,
    pub(crate) connected: broadcast::Sender<bool>,

    writer: tokio::task::JoinHandle<()>,
    reader: tokio::task::JoinHandle<()>,
}

impl Connection {
    pub(crate) async fn new(
        socket: TcpStream,
    ) -> (Self, ServerConnection, broadcast::Receiver<bool>) {
        let (inbound, incoming) = tokio::sync::mpsc::channel(32);
        let (outgoing, mut outbound) = tokio::sync::mpsc::channel::<Packets>(32);

        // NOTE: capacity 3 because *hopefully* max number of disconnect senders at one time
        let (ctx, mut crx) = broadcast::channel(3);

        let (reader, writer) = socket.into_split();

        // Write to Client
        let mut crxc = crx.resubscribe();
        let ctxc = ctx.clone();
        let writer = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = crxc.recv() => {
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
                                    ctxc.send(true).unwrap();
                                }
                                _ => {
                                    bytes.extend(packet.get_data());
                                }
                            }

                            let data =
                                bincode::encode_to_vec(RawPacket { id, data: bytes }, BC_CONFIG).unwrap();

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

            ctxc.send(true).unwrap();
        });

        // Read from Client
        let ctxc = ctx.clone();
        let outgoing_clone = outgoing.clone();
        let reader = tokio::spawn(async move {
            let mut state = ConnectionState::Handshake;

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
                        ctxc.send(true).unwrap();
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
                    let (length, size) =
                        bincode::decode_from_slice::<v32, _>(&buffer[index..], BC_CONFIG).unwrap();
                    let length = u32::from(length) as usize + size;

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
                    unsafe { packet_bytes.set_len(length) };

                    while remain > 0 {
                        trace!("Packet incomplete. {} bytes remaining", remain);

                        reader.readable().await.unwrap();
                        match reader.try_read(&mut packet_bytes[(length - remain)..]) {
                            Ok(n) => {
                                trace!("Read {} bytes", n);
                                remain -= n;
                            }
                            Err(e) => {
                                if e.kind() != std::io::ErrorKind::WouldBlock {
                                    error!("Error reading from connection: {}", e);
                                }
                                break;
                            }
                        }
                    }

                    {
                        use std::io::Write;
                        let mut file = std::fs::OpenOptions::new()
                            .create(true)
                            .write(true)
                            .open("packets.bin")
                            .unwrap();
                        file.write_all(&packet_bytes).unwrap();
                    }

                    // TODO: Compression
                    // TODO: Encryption
                    let (raw, rsize) =
                        bincode::decode_from_slice::<RawPacket, _>(&packet_bytes, BC_CONFIG)
                            .unwrap();
                    //index += rsize;

                    let packet = match state {
                        ConnectionState::Handshake => packets::serverbound::decode_handshaking(
                            u32::from(raw.id) as u8,
                            raw.data,
                        ),
                        ConnectionState::Status => {
                            packets::serverbound::decode_status(u32::from(raw.id) as u8, raw.data)
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

                    if process_packet(&packet, &mut state, &outgoing_clone).await {
                        inbound.send(packet).await.unwrap(); // FIXME: bruh
                    }
                }
            }
        });

        let crx = ctx.subscribe();

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
) -> bool {
    match packet {
        Packets::ServerboundHandshakingHandshake(packet) => {
            debug!(
                "Client connected with protocol version {}",
                u32::from(packet.protocol_version)
            );
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
        _ => {
            return true;
        }
    }
    false
}
