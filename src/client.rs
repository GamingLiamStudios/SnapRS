use crab_nbt::nbt;
use nom::{
    Parser,
    bytes::tag,
    number::be_u16,
};
use smol::net::TcpStream;
use tracing::{
    debug,
    info,
    trace,
    warn,
};
use uuid::Uuid;
use vek::{
    Vec2,
    Vec3,
};

use crate::{
    blocks::{
        BlockState,
        Blocks,
    },
    encode::{
        self,
        Generate,
    },
    packets::{
        self,
        COMPRESSION_MIN_SIZE,
        ClientConnection,
        DummyError,
        play::client::PosRelativeFlags,
    },
    parser::{
        construct_varint,
        parse_string,
        parse_varint,
    },
    text::TextComponent,
    world::World,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Gamemode {
    Survival = 0,
    Creative,
    Adventure,
    Spectator,
}

fn handle_plugin_message(
    channel: &str,
    data: &[u8],
) -> Vec<u8> {
    let mut buffer = Vec::new();
    match channel {
        "minecraft:brand" => {
            // Parse client data
            let (_, client_brand) =
                parse_string::<32767>(data).expect("Failed to parse brand channel");
            info!("Client has brand {client_brand}");

            _ = encode::bounded_string::<32767, DummyError>("SnapRS")
                .generate_in_place(&mut buffer);
        },
        channel => {
            debug!("Recieved data from unknown channel {channel}");
        },
    }

    buffer
}

async fn handle_configure(
    client: &mut ClientConnection,
    buffer: &mut Vec<u8>,
    name: &str,
) -> std::io::Result<i8> {
    use packets::configure::{
        ConfigurePacket,
        client,
    };

    let mut render_distance = 16;
    loop {
        packets::recv_packet(client, buffer).await?;
        match ConfigurePacket::parse(buffer) {
            Ok((_, ConfigurePacket::PluginMessage { channel, data })) => {
                // Respond with channel data
                let response = handle_plugin_message(channel, data);

                let packet = client::PluginMessage {
                    channel,
                    data: &response,
                };
                _ = client.write_packet(packet).await;
            },
            Ok((
                _,
                ConfigurePacket::Information {
                    locale: _,
                    view_distance,
                    chat_mode: _,
                    chat_colors: _,
                    enabled_skin: _,
                    main_hand: _,
                    text_filtering: _,
                    server_listings: _,
                },
            )) => {
                let mut packet = client::RegistryData::new();
                packet.add_entry(nbt!("minecraft:dimension_type", {
                    "type": "minecraft:dimension_type",
                    "value": [
                        {
                            "name": "snap_pp:overworld",
                            "id": 0,
                            "element": {
                                "has_skylight": true,
                                "has_ceiling": false,
                                "ultrawarm": false,
                                "natural": true,
                                "coordinate_scale": 1.0,
                                "bed_works": true,
                                "respawn_anchor_works": false,
                                "min_y": 0,
                                "height": 64,
                                "logical_height": 32,
                                "infiniburn": "#minecraft:dirt",
                                "effects": "minecraft:overworld",
                                "ambient_light": 0.0,
                                "piglin_safe": false,
                                "has_raids": true,
                                "monster_spawn_light_level": 0,
                                "monster_spawn_block_light_limit": 0,
                            }
                        }
                    ]
                }));
                packet.add_entry(nbt!("minecraft:worldgen/biome", {
                    "type": "minecraft:worldgen/biome",
                    "value": [
                        {
                            "name": "minecraft:beach",
                            "id": 0,
                            "element": {
                                "effects": {
                                    "sky_color": 7_907_327,
                                    "water_fog_color": 329_011,
                                    "fog_color": 12_638_463,
                                    "water_color": 4_159_204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.8,
                                "downfall": 0.4
                            }
                        },
                    ]
                }));
                _ = client.write_packet(packet).await;

                _ = client.write_packet(client::Finish).await;
                render_distance = view_distance;
            },
            Ok((_, ConfigurePacket::AckFinish)) => break,
            Ok((_, packet)) => {
                info!(?packet, "{name} sent config packet");
            },
            Err(nom::Err::Incomplete(_)) => {
                warn!("Client sent incomplete packet");
                return Err(std::io::ErrorKind::BrokenPipe.into());
            },
            Err(nom::Err::Error(error) | nom::Err::Failure(error)) => {
                warn!(?error, "Unknown Packet?");
            },
        }
    }

    Ok(render_distance)
}

async fn handle_login(
    client: &mut ClientConnection,
    buffer: &mut Vec<u8>,
) -> std::io::Result<(String, Uuid)> {
    use packets::login::{
        LoginPacket,
        client,
    };

    packets::recv_packet(client, buffer).await?;
    let Ok((_, LoginPacket::Start { name, uuid })) = LoginPacket::parse(buffer) else {
        return Err(std::io::ErrorKind::InvalidData.into());
    };
    let name = name.to_owned(); // Own it, since buffer isn't constant

    let packet = client::Compression {
        max_size: COMPRESSION_MIN_SIZE,
    };
    if let Err(error) = client.write_packet(packet).await {
        warn!(?error, "Failed to enable compression for packet");
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    client.compression = true;

    // TODO: Encryption flow

    let packet = client::Success {
        username: &name,
        uuid,
        properties: Vec::new(),
    };
    if let Err(error) = client.write_packet(packet).await {
        warn!(?error, "Failed to complete Login process");
        return Err(std::io::ErrorKind::InvalidData.into());
    }

    packets::recv_packet(client, buffer).await?;
    let Ok((_, LoginPacket::Ack)) = LoginPacket::parse(buffer) else {
        return Err(std::io::ErrorKind::InvalidData.into());
    };
    trace!("Got Login Ack");

    Ok((name, uuid))
}

#[allow(clippy::too_many_lines)]
pub async fn spawn(stream: TcpStream) {
    let mut client = ClientConnection::new(stream);

    // Read handshake packet
    // TODO: Custom logic for Legacy Ping
    let mut buffer = vec![0; 2048];
    if packets::recv_packet(&mut client, &mut buffer)
        .await
        .is_err()
    {
        warn!("Attemped connection from invalid client");
        return;
    }

    let Ok((_, (_, _, _server_address, _server_port, next_state))) = (
        tag(&construct_varint::<0x00>()[..]),
        tag(&construct_varint::<765>()[..]), // 1.20.4
        parse_string::<256>,
        be_u16(),
        parse_varint,
    )
        .parse(&buffer[..])
    else {
        return;
    };

    match next_state {
        1 => {
            // Status
            use packets::status::{
                StatusPacket,
                client,
            };

            loop {
                if packets::recv_packet(&mut client, &mut buffer)
                    .await
                    .is_err()
                {
                    return;
                }

                match StatusPacket::parse(&buffer) {
                    Ok((_, StatusPacket::Request)) => {
                        // TODO: Fetch server info
                        let response = client::StatusResponse {};
                        if client.write_packet(response).await.is_err() {
                            return;
                        }
                    },
                    Ok((_, StatusPacket::Ping(timestamp))) => {
                        let response = client::StatusPong { timestamp };
                        _ = client.write_packet(response).await;
                        return;
                    },
                    Err(_) => return,
                }
            }
        },
        2 => {
            // Login, handled outside of match
        },
        _ => return,
    }

    // Do Login
    let Ok((name, uuid)) = handle_login(&mut client, &mut buffer).await else {
        return;
    };

    // Do configuration flow
    // TODO: Add ResourcePack support
    // TODO: Actually use the info from here more
    let Ok(render_distance) = handle_configure(&mut client, &mut buffer, &name).await else {
        return;
    };

    info!(render_distance, ?uuid, "{name} Connected");

    // For the client to begin sending packets, we need to send 2 key packets
    // Login & SynchronizePosition
    // Only after these can we start sending chunking packets

    // Status
    use packets::play::{
        PlayPacket,
        client,
    };

    let mut world = World::new(0, 64);

    // Set world floor to dirt
    for x in -32..32 {
        for z in -32..32 {
            world.set_block(Vec3::new(x, 1, z), BlockState::Dirt);
        }
    }

    let packet = client::Login {
        eid:              0,
        last_death:       None,
        hardcore:         false,
        gamemode:         Gamemode::Creative,
        prev_gamemode:    None,
        dimensions:       vec!["snap_pp:overworld"],
        dimension_type:   "snap_pp:overworld",
        dimension_name:   "overworld",
        max_players:      20,
        view_distance:    8,
        sim_distance:     8,
        reduced_debug:    false,
        respawn_screen:   true,
        limited_crafting: false,
        hashed_seed:      0,
        is_debug:         false,
        is_flat:          true,
        portal_cooldown:  0,
    };
    _ = client.write_packet(packet).await;

    let packet = client::SyncPlayerPos {
        pos:         Vec3::new(0.0, 1.0, 0.0),
        rot:         Vec2::new(0.0, 0.0),
        flags:       PosRelativeFlags::empty(),
        teleport_id: 69,
    };
    _ = client.write_packet(packet).await;

    _ = client.write_packet(client::GameEvent::WaitForChunks).await;
    _ = client.write_packet(client::KeepAlive(69420)).await;

    _ = client
        .write_packet(client::CenterChunk {
            chunk_x: 0,
            chunk_z: 0,
        })
        .await;

    for z in -16..16 {
        for x in -16..16 {
            _ = client
                .write_packet(client::ChunkFull {
                    chunk_z: z,
                    chunk_x: x,
                    chunk:   world.get_chunk(Vec2::new(x, z)),
                })
                .await;
        }
    }

    loop {
        // TODO: Send Disconnect reason on failure
        if packets::recv_packet(&mut client, &mut buffer)
            .await
            .is_err()
        {
            break;
        }
        let Ok((_, packet)) = PlayPacket::parse(&buffer) else {
            break;
        };

        debug!(?packet);
    }
    info!("Done with {name}");
}
