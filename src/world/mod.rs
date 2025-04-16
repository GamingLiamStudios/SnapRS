use std::{
    collections::BTreeMap,
    range::Range,
};

pub use chunk::Chunk;
pub use entity::Entity;
use entity::{
    EntityData,
    EntityId,
};

use crate::blocks::BlockState;

pub mod chunk;
pub mod entity;

#[derive(Debug)]
pub struct World {
    height: Range<i32>,
    chunks: BTreeMap<(i32, i32), Chunk>,

    next_eid: u32,
    entities: BTreeMap<EntityId, EntityData>,
}

impl World {
    /// # Panics
    /// Will panic if the world height isn't valid
    #[must_use]
    pub fn new(
        min_y: i32,
        height: i32,
    ) -> Self {
        assert!(
            min_y % 16 == 0 && height % 16 == 0,
            "Height must be multiple of 16"
        );
        assert!(
            (-2032..2031).contains(&min_y),
            "min_y not within valid range"
        );
        assert!(
            (16..4064).contains(&height) && min_y + height < 2032,
            "Height not within range"
        );

        Self {
            height: Range {
                start: min_y,
                end:   min_y + height,
            },
            chunks: BTreeMap::new(),

            next_eid: 0,
            entities: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn get_chunk(
        &mut self,
        position: vek::Vec2<i32>,
    ) -> &mut Chunk {
        self.chunks
            .entry(position.into_tuple())
            .or_insert_with(|| Chunk::new(self.height.start, self.height.end))
    }

    #[must_use]
    pub fn get_block(
        &mut self,
        position: vek::Vec3<i32>,
    ) -> Option<BlockState> {
        if !self.height.contains(&position.y) {
            return None;
        }

        self.chunks
            .entry((position.x.div_floor(16), position.z.div_floor(16)))
            .or_insert_with(|| Chunk::new(self.height.start, self.height.end))
            .get_block(
                position
                    .with_x(position.x.abs() % 16)
                    .with_z(position.z.abs() % 16),
            )
    }

    pub fn set_block(
        &mut self,
        position: vek::Vec3<i32>,
        block: BlockState,
    ) -> Option<u16> {
        if !self.height.contains(&position.y) {
            return None;
        }

        self.chunks
            .entry((position.x.div_floor(16), position.z.div_floor(16)))
            .or_insert_with(|| Chunk::new(self.height.start, self.height.end))
            .set_block(
                position
                    .with_x(position.x.abs() % 16)
                    .with_z(position.z.abs() % 16),
                block,
            )
    }
}
