use bitflags::bitflags;
use crab_nbt::nbt;
use educe::Educe;
use vek::{
    Vec2,
    Vec3,
};

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
    world::{
        Chunk,
        Entity,
    },
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
    WaitForChunks,
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
            Self::WaitForChunks => (13u8, 0f32),
        }
        .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct KeepAlive(pub i64);

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

#[derive(Educe)]
#[educe(Debug)]
pub struct ChunkFull<'a> {
    pub chunk_x: i32,
    pub chunk_z: i32,

    #[educe(Debug(ignore))]
    pub chunk: &'a Chunk,
}

impl PacketBuilder<PlayError> for ChunkFull<'_> {
    const PACKET_ID: i32 = 0x25;
}

impl Generate<PlayError> for ChunkFull<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        (
            self.chunk_x,
            self.chunk_z,
            self.chunk,
            0u8,
            0u8,
            0u8,
            0u8,
            0u8,
            0u8,
            0u8,
        )
            .generate_in_place(buf)
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
    pub eid:        i32,
    pub last_death: Option<(&'a str, vek::Vec3<i32>)>,

    // TODO: Refactor hardcore into a gamemode (Hardcore Creative doesn't make sense :p)
    pub hardcore:      bool,
    pub gamemode:      Gamemode,
    pub prev_gamemode: Option<Gamemode>,

    pub dimensions:     Vec<&'a str>,
    pub dimension_type: &'a str,
    pub dimension_name: &'a str,

    pub max_players:   i32,
    pub view_distance: i32,
    pub sim_distance:  i32,

    pub reduced_debug:    bool,
    pub respawn_screen:   bool,
    pub limited_crafting: bool,

    pub hashed_seed: i64, // First 8 bytes of SHA-256 of world seed
    pub is_debug:    bool,
    pub is_flat:     bool,

    pub portal_cooldown: i32,
}

impl PacketBuilder<PlayError> for Login<'_> {
    const PACKET_ID: i32 = 0x29;
}

impl Generate<PlayError> for Login<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        let mut dimension_bytes = Vec::new();
        for dim in &self.dimensions {
            encode::bounded_string::<{ IDENTIFIER_MAX_LEN as usize }, PlayError>(dim)
                .generate_in_place(&mut dimension_bytes)?;
        }

        (
            self.eid,
            self.hardcore,
            encode::write_varint(
                i32::try_from(self.dimensions.len()).expect("Length was larger than i32::MAX"),
            ),
            dimension_bytes.as_slice(),
            encode::write_varint(self.max_players),
            encode::write_varint(self.view_distance),
            encode::write_varint(self.sim_distance),
            self.reduced_debug,
            self.respawn_screen,
            self.limited_crafting,
            encode::bounded_string::<{ IDENTIFIER_MAX_LEN as usize }, _>(self.dimension_type),
            encode::bounded_string::<{ IDENTIFIER_MAX_LEN as usize }, _>(self.dimension_name),
            self.hashed_seed,
            self.gamemode as u8,
            self.prev_gamemode.map_or(-1, |v| v as i8),
            self.is_debug,
            self.is_flat,
            self.last_death.is_some(),
            self.last_death.map(|(dimension, location)| {
                (
                    encode::bounded_string::<{ IDENTIFIER_MAX_LEN as usize }, _>(dimension),
                    encode::position(location),
                )
            }),
            encode::write_varint(self.portal_cooldown),
        )
            .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct CenterChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl PacketBuilder<PlayError> for CenterChunk {
    const PACKET_ID: i32 = 0x52;
}

impl Generate<PlayError> for CenterChunk {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        (
            encode::write_varint(self.chunk_x),
            encode::write_varint(self.chunk_z),
        )
            .generate_in_place(buf)
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct PosRelativeFlags : u8 {
        const X = 1 << 0;
        const Y = 1 << 1;
        const Z = 1 << 2;
        const PITCH = 1 << 3;
        const YAW = 1 << 4;
    }
}

#[derive(Debug)]
pub struct SyncPlayerPos {
    pub pos:   Vec3<f64>,
    pub rot:   Vec2<f32>,
    pub flags: PosRelativeFlags,

    pub teleport_id: i32,
}

impl PacketBuilder<PlayError> for SyncPlayerPos {
    const PACKET_ID: i32 = 0x3e;
}

impl Generate<PlayError> for SyncPlayerPos {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        (
            self.pos.x,
            self.pos.y,
            self.pos.z,
            self.rot.x,
            self.rot.y,
            self.flags.bits(),
            encode::write_varint(self.teleport_id),
        )
            .generate_in_place(buf)
    }
}

#[derive(Debug)]
pub struct SpawnEntity<'a> {
    entity: &'a Entity,
}

impl PacketBuilder<PlayError> for SpawnEntity<'_> {
    const PACKET_ID: i32 = 0x01;
}

impl Generate<PlayError> for SpawnEntity<'_> {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, PlayError> {
        let (eid, entity) = self.entity;
        (encode::write_varint(eid.into()), entity.uuid.as_u128()).generate_in_place(buf)
    }
}
