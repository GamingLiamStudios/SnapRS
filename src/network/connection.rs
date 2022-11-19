use std::sync::Arc;

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::prelude::*;

use log::{debug, error, trace, warn};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use crate::{
    config::CONFIG,
    packets::{self, serial, Packets},
};

use crate::packets::types::*;

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
    connected: broadcast::Sender<(bool, String)>,

    writer: tokio::task::JoinHandle<()>,
    reader: tokio::task::JoinHandle<()>,
}

impl Connection {
    pub(crate) async fn new(
        socket: TcpStream,
    ) -> (Self, ServerConnection, broadcast::Receiver<(bool, String)>) {
        let (inbound, incoming) = tokio::sync::mpsc::channel(32);
        let (outgoing, mut outbound) = tokio::sync::mpsc::channel::<Packets>(32);

        // How did I get this number? Spamming 'Refresh' in the server list until I didn't get a LAGGED error
        let (ctx, crx) = broadcast::channel(5);

        // Probably isn't needed but just in case
        // NOTE: Race Condition prevention
        let crx1 = crx.resubscribe();
        let crx2 = crx.resubscribe();

        let (reader, writer) = socket.into_split();

        let compressed = Arc::new(RwLock::new(false));

        // TODO: Figure out what to do with recv/send errors

        // Write to Client
        let ctxc = ctx.clone();
        let cc = compressed.clone();
        let writer = tokio::spawn(async move {
            let mut crx = crx1;
            let ctx = ctxc;
            let compressed = cc;
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

                            let mut should_enable_compression = false; // TODO: Something better
                            match packet {
                                Packets::ClientboundLoginDisconnect(packet) => {
                                    bytes.extend(serial::encode_to_vec(packet.as_ref()).unwrap());

                                    debug!("Disconnecting client: {}", String::from(packet.reason.value));
                                    ctx.send((true, "".to_string())).unwrap();
                                }
                                Packets::ClientboundLoginSetCompression(_) => {
                                    should_enable_compression = true;
                                    bytes.extend(packet.get_data());
                                }
                                _ => {
                                    bytes.extend(packet.get_data());
                                }
                            }

                            // Normal packet
                            let len = (bytes.len() + 1) as u32;
                            let mut data = Vec::with_capacity(bytes.len() + 1 + v32::byte_size(len));
                            data.extend(serial::encode_to_vec(&v32::from(len)).unwrap());
                            data.push(id);
                            data.extend(bytes);

                            if *compressed.read().await {
                                let mut compressed_len = 0;
                                if len > CONFIG.network.advanced.compression_threshold {
                                    let mut zlib = ZlibEncoder::new(Vec::new(), Compression::new(CONFIG.network.advanced.compression_level));
                                    zlib.write_all(&data[v32::byte_size(len)..]).unwrap();
                                    let compressed_data = zlib.finish().unwrap();

                                    // If our compressed data is smaller, use it
                                    if compressed_data.len() < len as usize {
                                        compressed_len = compressed_data.len() as u32;
                                        data = compressed_data;
                                    }
                                }

                                // Compressed packet
                                let len = data.len() + v32::byte_size(compressed_len);
                                let mut bytes = Vec::with_capacity(v32::byte_size(len as u32) + len);
                                bytes.extend(serial::encode_to_vec(&v32::from(len as u32)).unwrap());
                                bytes.extend(serial::encode_to_vec(&v32::from(compressed_len)).unwrap());
                                bytes.extend(data);

                                data = bytes;
                            }

                            if data.len() > 2097151 {
                                error!("Packet too large! {}", data.len());
                                ctx.send((true, format!("Server tried sending Packet size {}", data.len()))).unwrap();
                                continue;
                            }

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

                            /*
                                As SetCompression is never recieved and packets are only compressed AFTER,
                                we *shouldn't* have to worry about syncing this with the reader.
                            */
                            if should_enable_compression {
                                *compressed.write().await = true;
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

            // Take ownership of 'compressed' as it is not used after this is spawned.

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

                // TODO: Encryption
                let read = match reader.try_read(&mut buffer) {
                    Ok(0) => {
                        //trace!("Connection closed");
                        ctx.send((true, "".to_string())).unwrap();
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
                    let (length, lsize) =
                        serial::decode_from_slice::<v32>(&buffer[index..]).unwrap();
                    let length = u32::from(length) as usize;
                    index += lsize;

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
                                warn!("Connection close while reading packet");
                                ctx.send((true, "".to_string())).unwrap();
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

                    let id;
                    let data;

                    if *compressed.read().await {
                        let (compressed_len, clsize) =
                            serial::decode_from_slice::<v32>(&packet_bytes).unwrap();
                        let compressed_len = u32::from(compressed_len) as usize;

                        if compressed_len > 0 {
                            let mut bytes = ZlibDecoder::new(&packet_bytes[clsize..]).bytes();
                            id = bytes.next().unwrap().unwrap();

                            let mut d = bytes.map(|x| x.unwrap()).collect::<Vec<u8>>();
                            d.shrink_to(compressed_len);
                            data = d;
                        } else {
                            // Start with index of 1 as the expected clsize(0) is guaranteed to be 1 byte.
                            id = packet_bytes[1];
                            data = packet_bytes[2..].to_vec();
                        }
                    } else {
                        id = packet_bytes[0];
                        data = packet_bytes[1..].to_vec();
                    }

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

        self.connected
            .send((true, "Connection Closed by Server".to_string()))
            .unwrap();

        self.writer.await.unwrap();
        self.reader.await.unwrap();
    }
}

// Returns true if the packet should be sent to the server
async fn process_packet(
    packet: &Packets,
    state: &mut ConnectionState,
    outgoing: &Sender<Packets>,
    close_sender: &broadcast::Sender<(bool, String)>,
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
                close_sender.send((true, "".to_string())).unwrap();
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
            outgoing
                .send(Packets::from(
                    packets::clientbound::login_packets::SetCompression {
                        threshold: v32::from(CONFIG.network.advanced.compression_threshold),
                    },
                ))
                .await
                .unwrap();
        }
        _ => {
            return true;
        }
    }
    false
}
