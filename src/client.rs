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

#[allow(clippy::too_many_lines)]
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
                            "name": "minecraft:badlands",
                            "id": 0,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.badlands",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7254527,
                                    "grass_color": 9470285,
                                    "foliage_color": 10387789,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:bamboo_jungle",
                            "id": 1,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.bamboo_jungle",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7842047,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.95,
                                "downfall": 0.9
                            }
                        },
                        {
                            "name": "minecraft:basalt_deltas",
                            "id": 2,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.nether.basalt_deltas",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7254527,
                                    "ambient_sound": "minecraft:ambient.basalt_deltas.loop",
                                    "additions_sound": {
                                        "sound": "minecraft:ambient.basalt_deltas.additions",
                                        "tick_chance": 0.0111
                                    },
                                    "particle": {
                                        "probability": 0.118093334,
                                        "options": {
                                            "type": "minecraft:white_ash"
                                        }
                                    },
                                    "water_fog_color": 329011,
                                    "fog_color": 6840176,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.basalt_deltas.mood",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:beach",
                            "id": 3,
                            "element": {
                                "effects": {
                                    "sky_color": 7907327,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
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
                        {
                            "name": "minecraft:birch_forest",
                            "id": 4,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.forest",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8037887,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.6,
                                "downfall": 0.6
                            }
                        },
                        {
                            "name": "minecraft:cherry_grove",
                            "id": 5,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.cherry_grove",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8103167,
                                    "grass_color": 11983713,
                                    "foliage_color": 11983713,
                                    "water_fog_color": 6141935,
                                    "fog_color": 12638463,
                                    "water_color": 6141935,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.8
                            }
                        },
                        {
                            "name": "minecraft:cold_ocean",
                            "id": 6,
                            "element": {
                                "effects": {
                                    "sky_color": 8103167,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4020182,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:crimson_forest",
                            "id": 7,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.nether.crimson_forest",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7254527,
                                    "ambient_sound": "minecraft:ambient.crimson_forest.loop",
                                    "additions_sound": {
                                        "sound": "minecraft:ambient.crimson_forest.additions",
                                        "tick_chance": 0.0111
                                    },
                                    "particle": {
                                        "probability": 0.025,
                                        "options": {
                                            "type": "minecraft:crimson_spore"
                                        }
                                    },
                                    "water_fog_color": 329011,
                                    "fog_color": 3343107,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.crimson_forest.mood",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:dark_forest",
                            "id": 8,
                            "element": {
                                "effects": {
                                    "grass_color_modifier": "dark_forest",
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.forest",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7972607,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.7,
                                "downfall": 0.8
                            }
                        },
                        {
                            "name": "minecraft:deep_cold_ocean",
                            "id": 9,
                            "element": {
                                "effects": {
                                    "sky_color": 8103167,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4020182,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:deep_dark",
                            "id": 10,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.deep_dark",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7907327,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
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
                        {
                            "name": "minecraft:deep_frozen_ocean",
                            "id": 11,
                            "element": {
                                "effects": {
                                    "sky_color": 8103167,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 3750089,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.5,
                                "temperature_modifier": "frozen"
                            }
                        },
                        {
                            "name": "minecraft:deep_lukewarm_ocean",
                            "id": 12,
                            "element": {
                                "effects": {
                                    "sky_color": 8103167,
                                    "water_fog_color": 267827,
                                    "fog_color": 12638463,
                                    "water_color": 4566514,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:deep_ocean",
                            "id": 13,
                            "element": {
                                "effects": {
                                    "sky_color": 8103167,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:desert",
                            "id": 14,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.desert",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7254527,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:dripstone_caves",
                            "id": 15,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.dripstone_caves",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7907327,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
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
                        {
                            "name": "minecraft:end_barrens",
                            "id": 16,
                            "element": {
                                "effects": {
                                    "sky_color": 0,
                                    "water_fog_color": 329011,
                                    "fog_color": 10518688,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:end_highlands",
                            "id": 17,
                            "element": {
                                "effects": {
                                    "sky_color": 0,
                                    "water_fog_color": 329011,
                                    "fog_color": 10518688,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:end_midlands",
                            "id": 18,
                            "element": {
                                "effects": {
                                    "sky_color": 0,
                                    "water_fog_color": 329011,
                                    "fog_color": 10518688,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:eroded_badlands",
                            "id": 19,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.badlands",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7254527,
                                    "grass_color": 9470285,
                                    "foliage_color": 10387789,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:flower_forest",
                            "id": 20,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.flower_forest",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7972607,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.7,
                                "downfall": 0.8
                            }
                        },
                        {
                            "name": "minecraft:forest",
                            "id": 21,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.forest",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7972607,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.7,
                                "downfall": 0.8
                            }
                        },
                        {
                            "name": "minecraft:frozen_ocean",
                            "id": 22,
                            "element": {
                                "effects": {
                                    "sky_color": 8364543,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 3750089,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.0,
                                "downfall": 0.5,
                                "temperature_modifier": "frozen"
                            }
                        },
                        {
                            "name": "minecraft:frozen_peaks",
                            "id": 23,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.frozen_peaks",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8756735,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.7,
                                "downfall": 0.9
                            }
                        },
                        {
                            "name": "minecraft:frozen_river",
                            "id": 24,
                            "element": {
                                "effects": {
                                    "sky_color": 8364543,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 3750089,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.0,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:grove",
                            "id": 25,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.grove",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8495359,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.2,
                                "downfall": 0.8
                            }
                        },
                        {
                            "name": "minecraft:ice_spikes",
                            "id": 26,
                            "element": {
                                "effects": {
                                    "sky_color": 8364543,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.0,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:jagged_peaks",
                            "id": 27,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.jagged_peaks",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8756735,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.7,
                                "downfall": 0.9
                            }
                        },
                        {
                            "name": "minecraft:jungle",
                            "id": 28,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.jungle",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7842047,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.95,
                                "downfall": 0.9
                            }
                        },
                        {
                            "name": "minecraft:lukewarm_ocean",
                            "id": 29,
                            "element": {
                                "effects": {
                                    "sky_color": 8103167,
                                    "water_fog_color": 267827,
                                    "fog_color": 12638463,
                                    "water_color": 4566514,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:lush_caves",
                            "id": 30,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.lush_caves",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8103167,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:mangrove_swamp",
                            "id": 31,
                            "element": {
                                "effects": {
                                    "grass_color_modifier": "swamp",
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.swamp",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7907327,
                                    "foliage_color": 9285927,
                                    "water_fog_color": 5077600,
                                    "fog_color": 12638463,
                                    "water_color": 3832426,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.8,
                                "downfall": 0.9
                            }
                        },
                        {
                            "name": "minecraft:meadow",
                            "id": 32,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.meadow",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8103167,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 937679,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.8
                            }
                        },
                        {
                            "name": "minecraft:mushroom_fields",
                            "id": 33,
                            "element": {
                                "effects": {
                                    "sky_color": 7842047,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.9,
                                "downfall": 1.0
                            }
                        },
                        {
                            "name": "minecraft:nether_wastes",
                            "id": 34,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.nether.nether_wastes",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7254527,
                                    "ambient_sound": "minecraft:ambient.nether_wastes.loop",
                                    "additions_sound": {
                                        "sound": "minecraft:ambient.nether_wastes.additions",
                                        "tick_chance": 0.0111
                                    },
                                    "water_fog_color": 329011,
                                    "fog_color": 3344392,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.nether_wastes.mood",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:ocean",
                            "id": 35,
                            "element": {
                                "effects": {
                                    "sky_color": 8103167,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:old_growth_birch_forest",
                            "id": 36,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.forest",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8037887,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.6,
                                "downfall": 0.6
                            }
                        },
                        {
                            "name": "minecraft:old_growth_pine_taiga",
                            "id": 37,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.old_growth_taiga",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8168447,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.3,
                                "downfall": 0.8
                            }
                        },
                        {
                            "name": "minecraft:old_growth_spruce_taiga",
                            "id": 38,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.old_growth_taiga",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8233983,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.25,
                                "downfall": 0.8
                            }
                        },
                        {
                            "name": "minecraft:plains",
                            "id": 39,
                            "element": {
                                "effects": {
                                    "sky_color": 7907327,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
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
                        {
                            "name": "minecraft:river",
                            "id": 40,
                            "element": {
                                "effects": {
                                    "sky_color": 8103167,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:savanna",
                            "id": 41,
                            "element": {
                                "effects": {
                                    "sky_color": 7254527,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:savanna_plateau",
                            "id": 42,
                            "element": {
                                "effects": {
                                    "sky_color": 7254527,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:small_end_islands",
                            "id": 43,
                            "element": {
                                "effects": {
                                    "sky_color": 0,
                                    "water_fog_color": 329011,
                                    "fog_color": 10518688,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:snowy_beach",
                            "id": 44,
                            "element": {
                                "effects": {
                                    "sky_color": 8364543,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4020182,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.05,
                                "downfall": 0.3
                            }
                        },
                        {
                            "name": "minecraft:snowy_plains",
                            "id": 45,
                            "element": {
                                "effects": {
                                    "sky_color": 8364543,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.0,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:snowy_slopes",
                            "id": 46,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.snowy_slopes",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 8560639,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.3,
                                "downfall": 0.9
                            }
                        },
                        {
                            "name": "minecraft:snowy_taiga",
                            "id": 47,
                            "element": {
                                "effects": {
                                    "sky_color": 8625919,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4020182,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.4
                            }
                        },
                        {
                            "name": "minecraft:soul_sand_valley",
                            "id": 48,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.nether.soul_sand_valley",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7254527,
                                    "ambient_sound": "minecraft:ambient.soul_sand_valley.loop",
                                    "additions_sound": {
                                        "sound": "minecraft:ambient.soul_sand_valley.additions",
                                        "tick_chance": 0.0111
                                    },
                                    "particle": {
                                        "probability": 0.00625,
                                        "options": {
                                            "type": "minecraft:ash"
                                        }
                                    },
                                    "water_fog_color": 329011,
                                    "fog_color": 1787717,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.soul_sand_valley.mood",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:sparse_jungle",
                            "id": 49,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.sparse_jungle",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7842047,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.95,
                                "downfall": 0.8
                            }
                        },
                        {
                            "name": "minecraft:stony_peaks",
                            "id": 50,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.stony_peaks",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7776511,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 1.0,
                                "downfall": 0.3
                            }
                        },
                        {
                            "name": "minecraft:stony_shore",
                            "id": 51,
                            "element": {
                                "effects": {
                                    "sky_color": 8233727,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.2,
                                "downfall": 0.3
                            }
                        },
                        {
                            "name": "minecraft:sunflower_plains",
                            "id": 52,
                            "element": {
                                "effects": {
                                    "sky_color": 7907327,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
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
                        {
                            "name": "minecraft:swamp",
                            "id": 53,
                            "element": {
                                "effects": {
                                    "grass_color_modifier": "swamp",
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.swamp",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7907327,
                                    "foliage_color": 6975545,
                                    "water_fog_color": 2302743,
                                    "fog_color": 12638463,
                                    "water_color": 6388580,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.8,
                                "downfall": 0.9
                            }
                        },
                        {
                            "name": "minecraft:taiga",
                            "id": 54,
                            "element": {
                                "effects": {
                                    "sky_color": 8233983,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.25,
                                "downfall": 0.8
                            }
                        },
                        {
                            "name": "minecraft:the_end",
                            "id": 55,
                            "element": {
                                "effects": {
                                    "sky_color": 0,
                                    "water_fog_color": 329011,
                                    "fog_color": 10518688,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:the_void",
                            "id": 56,
                            "element": {
                                "effects": {
                                    "sky_color": 8103167,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:warm_ocean",
                            "id": 57,
                            "element": {
                                "effects": {
                                    "sky_color": 8103167,
                                    "water_fog_color": 270131,
                                    "fog_color": 12638463,
                                    "water_color": 4445678,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.5,
                                "downfall": 0.5
                            }
                        },
                        {
                            "name": "minecraft:warped_forest",
                            "id": 58,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.nether.warped_forest",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7254527,
                                    "ambient_sound": "minecraft:ambient.warped_forest.loop",
                                    "additions_sound": {
                                        "sound": "minecraft:ambient.warped_forest.additions",
                                        "tick_chance": 0.0111
                                    },
                                    "particle": {
                                        "probability": 0.01428,
                                        "options": {
                                            "type": "minecraft:warped_spore"
                                        }
                                    },
                                    "water_fog_color": 329011,
                                    "fog_color": 1705242,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.warped_forest.mood",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:windswept_forest",
                            "id": 59,
                            "element": {
                                "effects": {
                                    "sky_color": 8233727,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.2,
                                "downfall": 0.3
                            }
                        },
                        {
                            "name": "minecraft:windswept_gravelly_hills",
                            "id": 60,
                            "element": {
                                "effects": {
                                    "sky_color": 8233727,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.2,
                                "downfall": 0.3
                            }
                        },
                        {
                            "name": "minecraft:windswept_hills",
                            "id": 61,
                            "element": {
                                "effects": {
                                    "sky_color": 8233727,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 1,
                                "temperature": 0.2,
                                "downfall": 0.3
                            }
                        },
                        {
                            "name": "minecraft:windswept_savanna",
                            "id": 62,
                            "element": {
                                "effects": {
                                    "sky_color": 7254527,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        },
                        {
                            "name": "minecraft:wooded_badlands",
                            "id": 63,
                            "element": {
                                "effects": {
                                    "music": {
                                        "replace_current_music": 0,
                                        "max_delay": 24000,
                                        "sound": "minecraft:music.overworld.badlands",
                                        "min_delay": 12000
                                    },
                                    "sky_color": 7254527,
                                    "grass_color": 9470285,
                                    "foliage_color": 10387789,
                                    "water_fog_color": 329011,
                                    "fog_color": 12638463,
                                    "water_color": 4159204,
                                    "mood_sound": {
                                        "tick_delay": 6000,
                                        "offset": 2.0,
                                        "sound": "minecraft:ambient.cave",
                                        "block_search_extent": 8
                                    }
                                },
                                "has_precipitation": 0,
                                "temperature": 2.0,
                                "downfall": 0.0
                            }
                        }
                    ]
                }));
                packet.add_entry(nbt!("minecraft:damage_type", {
                    "type": "minecraft:damage_type",
                    "value": [
                        {
                            "name": "minecraft:arrow",
                            "id": 0,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "arrow"
                            }
                        },
                        {
                            "name": "minecraft:bad_respawn_point",
                            "id": 1,
                            "element": {
                                "scaling": "always",
                                "exhaustion": 0.1,
                                "message_id": "badRespawnPoint",
                                "death_message_type": "intentional_game_design"
                            }
                        },
                        {
                            "name": "minecraft:cactus",
                            "id": 2,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "cactus"
                            }
                        },
                        {
                            "name": "minecraft:cramming",
                            "id": 3,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "cramming"
                            }
                        },
                        {
                            "name": "minecraft:dragon_breath",
                            "id": 4,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "dragonBreath"
                            }
                        },
                        {
                            "name": "minecraft:drown",
                            "id": 5,
                            "element": {
                                "effects": "drowning",
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "drown"
                            }
                        },
                        {
                            "name": "minecraft:dry_out",
                            "id": 6,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "dryout"
                            }
                        },
                        {
                            "name": "minecraft:explosion",
                            "id": 7,
                            "element": {
                                "scaling": "always",
                                "exhaustion": 0.1,
                                "message_id": "explosion"
                            }
                        },
                        {
                            "name": "minecraft:fall",
                            "id": 8,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "fall",
                                "death_message_type": "fall_variants"
                            }
                        },
                        {
                            "name": "minecraft:falling_anvil",
                            "id": 9,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "anvil"
                            }
                        },
                        {
                            "name": "minecraft:falling_block",
                            "id": 10,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "fallingBlock"
                            }
                        },
                        {
                            "name": "minecraft:falling_stalactite",
                            "id": 11,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "fallingStalactite"
                            }
                        },
                        {
                            "name": "minecraft:fireball",
                            "id": 12,
                            "element": {
                                "effects": "burning",
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "fireball"
                            }
                        },
                        {
                            "name": "minecraft:fireworks",
                            "id": 13,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "fireworks"
                            }
                        },
                        {
                            "name": "minecraft:fly_into_wall",
                            "id": 14,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "flyIntoWall"
                            }
                        },
                        {
                            "name": "minecraft:freeze",
                            "id": 15,
                            "element": {
                                "effects": "freezing",
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "freeze"
                            }
                        },
                        {
                            "name": "minecraft:generic",
                            "id": 16,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "generic"
                            }
                        },
                        {
                            "name": "minecraft:generic_kill",
                            "id": 17,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "genericKill"
                            }
                        },
                        {
                            "name": "minecraft:hot_floor",
                            "id": 18,
                            "element": {
                                "effects": "burning",
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "hotFloor"
                            }
                        },
                        {
                            "name": "minecraft:in_fire",
                            "id": 19,
                            "element": {
                                "effects": "burning",
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "inFire"
                            }
                        },
                        {
                            "name": "minecraft:in_wall",
                            "id": 20,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "inWall"
                            }
                        },
                        {
                            "name": "minecraft:indirect_magic",
                            "id": 21,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "indirectMagic"
                            }
                        },
                        {
                            "name": "minecraft:lava",
                            "id": 22,
                            "element": {
                                "effects": "burning",
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "lava"
                            }
                        },
                        {
                            "name": "minecraft:lightning_bolt",
                            "id": 23,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "lightningBolt"
                            }
                        },
                        {
                            "name": "minecraft:magic",
                            "id": 24,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "magic"
                            }
                        },
                        {
                            "name": "minecraft:mob_attack",
                            "id": 25,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "mob"
                            }
                        },
                        {
                            "name": "minecraft:mob_attack_no_aggro",
                            "id": 26,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "mob"
                            }
                        },
                        {
                            "name": "minecraft:mob_projectile",
                            "id": 27,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "mob"
                            }
                        },
                        {
                            "name": "minecraft:on_fire",
                            "id": 28,
                            "element": {
                                "effects": "burning",
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "onFire"
                            }
                        },
                        {
                            "name": "minecraft:out_of_world",
                            "id": 29,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "outOfWorld"
                            }
                        },
                        {
                            "name": "minecraft:outside_border",
                            "id": 30,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "outsideBorder"
                            }
                        },
                        {
                            "name": "minecraft:player_attack",
                            "id": 31,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "player"
                            }
                        },
                        {
                            "name": "minecraft:player_explosion",
                            "id": 32,
                            "element": {
                                "scaling": "always",
                                "exhaustion": 0.1,
                                "message_id": "explosion.player"
                            }
                        },
                        {
                            "name": "minecraft:sonic_boom",
                            "id": 33,
                            "element": {
                                "scaling": "always",
                                "exhaustion": 0.0,
                                "message_id": "sonic_boom"
                            }
                        },
                        {
                            "name": "minecraft:stalagmite",
                            "id": 34,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "stalagmite"
                            }
                        },
                        {
                            "name": "minecraft:starve",
                            "id": 35,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "starve"
                            }
                        },
                        {
                            "name": "minecraft:sting",
                            "id": 36,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "sting"
                            }
                        },
                        {
                            "name": "minecraft:sweet_berry_bush",
                            "id": 37,
                            "element": {
                                "effects": "poking",
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "sweetBerryBush"
                            }
                        },
                        {
                            "name": "minecraft:thorns",
                            "id": 38,
                            "element": {
                                "effects": "thorns",
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "thorns"
                            }
                        },
                        {
                            "name": "minecraft:thrown",
                            "id": 39,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "thrown"
                            }
                        },
                        {
                            "name": "minecraft:trident",
                            "id": 40,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "trident"
                            }
                        },
                        {
                            "name": "minecraft:unattributed_fireball",
                            "id": 41,
                            "element": {
                                "effects": "burning",
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "onFire"
                            }
                        },
                        {
                            "name": "minecraft:wither",
                            "id": 42,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.0,
                                "message_id": "wither"
                            }
                        },
                        {
                            "name": "minecraft:wither_skull",
                            "id": 43,
                            "element": {
                                "scaling": "when_caused_by_living_non_player",
                                "exhaustion": 0.1,
                                "message_id": "witherSkull"
                            }
                        }
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
    for y in 0..16 {
        for x in -64..=64 {
            for z in -64..=64 {
                world.set_block(Vec3::new(x, y, z), BlockState::Dirt);
            }
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
        pos:         Vec3::new(0.0, 32.0, 0.0),
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

    for z in -6..=6 {
        for x in -6..=6 {
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
