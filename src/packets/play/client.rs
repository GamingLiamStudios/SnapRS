use super::PlayError;
use crate::{
    client::Gamemode,
    encode::{
        self,
        Generate,
    },
    packets::PacketBuilder,
    parser::IDENTIFIER_MAX_LEN,
    text::{
        self,
        TextComponent,
    },
    world::Chunk,
};

// TODO: Create BundleScope interface
#[derive(Debug)]
pub struct BundleDelimiter;

impl PacketBuilder<PlayError> for BundleDelimiter {
    const PACKET_ID: i32 = 0x00;
}

impl Generate<PlayError> for BundleDelimiter {
    fn generate_in_place(
        &self,
        _buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        Ok(0)
    }
}

#[derive(Debug)]
pub struct AckBlockChange;

impl PacketBuilder<PlayError> for AckBlockChange {
    const PACKET_ID: i32 = 0x05;
}

impl Generate<PlayError> for AckBlockChange {
    fn generate_in_place(
        &self,
        _buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        Ok(0)
    }
}

#[derive(Debug)]
pub struct BlockAction {
    position:     vek::Vec3<i32>,
    action_id:    u8,
    action_param: u8,
    block_type:   i32,
}

impl PacketBuilder<PlayError> for BlockAction {
    const PACKET_ID: i32 = 0x08;
}

impl Generate<PlayError> for BlockAction {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        (
            encode::position(self.position),
            self.action_id,
            self.action_param,
            encode::write_varint(self.block_type),
        )
            .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct BlockUpdate {
    position: vek::Vec3<i32>,
    block_id: i32,
}

impl PacketBuilder<PlayError> for BlockUpdate {
    const PACKET_ID: i32 = 0x09;
}

impl Generate<PlayError> for BlockUpdate {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        (
            encode::position(self.position),
            encode::write_varint(self.block_id),
        )
            .generate_in_place(buf)
    }
}

// TODO: ChunkBatch metastruct
#[derive(Debug)]
pub struct ChunkBatchFinish(i32);

impl PacketBuilder<PlayError> for ChunkBatchFinish {
    const PACKET_ID: i32 = 0x0c;
}

impl Generate<PlayError> for ChunkBatchFinish {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        encode::write_varint(self.0).generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct ChunkBatchBegin;

impl PacketBuilder<PlayError> for ChunkBatchBegin {
    const PACKET_ID: i32 = 0x0d;
}

impl Generate<PlayError> for ChunkBatchBegin {
    fn generate_in_place(
        &self,
        _buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        Ok(0)
    }
}

#[derive(Debug)]
pub struct PluginMessage<'a> {
    pub channel: &'a str,
    pub data:    &'a [u8],
}

impl PacketBuilder<PlayError> for PluginMessage<'_> {
    const PACKET_ID: i32 = 0x18;
}

impl Generate<PlayError> for PluginMessage<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        if self.data.len() > 1_048_576 {
            return Err(PlayError::OversizedPluginData);
        }

        (
            encode::bounded_string::<{ IDENTIFIER_MAX_LEN as usize }, _>(self.channel),
            self.data,
        )
            .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct Disconnect<'a> {
    pub reason: &'a TextComponent,
}

impl PacketBuilder<PlayError> for Disconnect<'_> {
    const PACKET_ID: i32 = 0x1b;
}

impl Generate<PlayError> for Disconnect<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        text::write_text_component(self.reason).generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct UnloadChunk {
    chunk_x: i32,
    chunk_z: i32,
}

impl PacketBuilder<PlayError> for UnloadChunk {
    const PACKET_ID: i32 = 0x1f;
}

impl Generate<PlayError> for UnloadChunk {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        (self.chunk_z, self.chunk_x).generate_in_place(buf)
    }
}

#[derive(Debug)]
pub enum DemoType {
    WelcomeToDemo,
    Movement,
    Jump,
    Inventory,
    EndDemo,
}

#[derive(Debug)]
pub enum GameEvent {
    NoRespawnAvailable,
    EndRain,
    StartRain,
    GameMode(Gamemode),
    WinGame { credits: bool },
    Demo(DemoType),
    ArrowHit,
    RainLevel(f32),
    ThunderLevel(f32),
    PufferfishSting,
    ElderGuardian,
    DisableRespawnScreen(bool),
    EnableLimitedCraft(bool),
    WairForChunks,
}

impl PacketBuilder<PlayError> for GameEvent {
    const PACKET_ID: i32 = 0x20;
}

impl Generate<PlayError> for GameEvent {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        match self {
            Self::NoRespawnAvailable => (0u8, 0f32),
            Self::EndRain => (1u8, 0f32),
            Self::StartRain => (2u8, 0f32),
            Self::GameMode(gamemode) => (3u8, match gamemode {
                Gamemode::Survival => 0f32,
                Gamemode::Creative => 1f32,
                Gamemode::Adventure => 2f32,
                Gamemode::Spectator => 3f32,
            }),
            Self::WinGame { credits } => (4u8, if *credits { 1f32 } else { 0f32 }),
            Self::Demo(demotype) => (5u8, match demotype {
                DemoType::WelcomeToDemo => 0f32,
                DemoType::Movement => 101f32,
                DemoType::Jump => 102f32,
                DemoType::Inventory => 103f32,
                DemoType::EndDemo => 104f32,
            }),
            Self::ArrowHit => (6u8, 0f32),
            Self::RainLevel(level) => (7u8, *level),
            Self::ThunderLevel(level) => (8u8, *level),
            Self::PufferfishSting => (9u8, 0f32),
            Self::ElderGuardian => (10u8, 0f32),
            Self::DisableRespawnScreen(disabled) => (11u8, if *disabled { 1f32 } else { 0f32 }),
            Self::EnableLimitedCraft(enabled) => (12u8, if *enabled { 1f32 } else { 0f32 }),
            Self::WairForChunks => (13u8, 0f32),
        }
        .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct KeepAlive(i64);

impl PacketBuilder<PlayError> for KeepAlive {
    const PACKET_ID: i32 = 0x24;
}

impl Generate<PlayError> for KeepAlive {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        self.0.generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct ChunkFull<'a> {
    chunk_x: i32,
    chunk_z: i32,

    chunk: &'a Chunk,
}

impl PacketBuilder<PlayError> for ChunkFull<'_> {
    const PACKET_ID: i32 = 0x25;
}

impl Generate<PlayError> for ChunkFull<'_> {
    fn generate_in_place(
        &self,
        _buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        todo!()
        // TODO: This absolute unit of a packet
    }
}

#[derive(Debug)]
pub struct ChunkLight<'a> {
    chunk_x: i32,
    chunk_z: i32,

    chunk: &'a Chunk,
}

impl PacketBuilder<PlayError> for ChunkLight<'_> {
    const PACKET_ID: i32 = 0x28;
}

impl Generate<PlayError> for ChunkLight<'_> {
    fn generate_in_place(
        &self,
        _buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        todo!()
        // TODO: This absolute unit of a packet
    }
}

#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct Login<'a> {
    eid:        i32,
    last_death: Option<(&'a str, vek::Vec3<i32>)>,

    // TODO: Refactor hardcore into a gamemode (Hardcore Creative doesn't make sense :p)
    hardcore:      bool,
    gamemode:      Gamemode,
    prev_gamemode: Option<Gamemode>,

    dimensions:     Vec<&'a str>,
    dimension_type: &'a str,
    dimension_name: &'a str,

    max_players:   i32,
    view_distance: i32,
    sim_distance:  i32,

    reduced_debug:    bool,
    respawn_screen:   bool,
    limited_crafting: bool,

    hashed_seed: i64, // First 8 bytes of SHA-256 of world seed
    is_debug:    bool,
    is_flat:     bool,

    portal_cooldown: i32,
}

impl PacketBuilder<PlayError> for Login<'_> {
    const PACKET_ID: i32 = 0x29;
}

impl Generate<PlayError> for Login<'_> {
    fn generate_in_place(
        &self,
        _buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        todo!()
        // TODO: This absolute unit of a packet
    }
}
