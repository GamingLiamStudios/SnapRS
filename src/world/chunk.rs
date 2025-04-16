use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    range::Range,
};

use crab_nbt::nbt;

use crate::{
    blocks::BlockState,
    encode::{
        self,
        EncodeError,
        Generate,
    },
};

#[derive(Debug, Clone)]
pub struct ChunkSection {
    next:  u16,
    freed: BTreeSet<u16>,

    blocks:  u16,
    palette: BTreeMap<BlockState, u16>,
    data:    [u16; 16 * 16 * 16],
}

impl Default for ChunkSection {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkSection {
    #[must_use]
    pub fn new() -> Self {
        let mut palette = BTreeMap::new();
        palette.insert(BlockState::Air, 0);
        Self {
            next: 1,
            freed: BTreeSet::new(),
            palette,
            blocks: 0,
            data: [0; 16 * 16 * 16],
        }
    }

    pub fn compact(&mut self) {
        // TODO: Optimize, as it seems like it'll be slow
        while let Some(free_id) = self.freed.pop_last() {
            // Move everything in-front of free_idx back by one
            for block in &mut self.data {
                if *block > free_id {
                    *block -= 1;
                }
            }

            for id in self.palette.values_mut() {
                if *id > free_id {
                    *id -= 1;
                }
            }
        }
    }

    /// # Panics
    /// Will panic if (x, y, z) are >= 16
    pub fn set_block(
        &mut self,
        position: vek::Vec3<u8>,
        block: BlockState,
    ) -> Option<u16> {
        if !(position.x < 16 && position.y < 16 && position.z < 16) {
            return None;
        }

        let prev_block = self.get_block(position)?;
        match (block, prev_block) {
            (
                BlockState::Air | BlockState::CaveAir | BlockState::VoidAir,
                BlockState::Air | BlockState::CaveAir | BlockState::VoidAir,
            ) => (),
            (_, BlockState::Air | BlockState::CaveAir | BlockState::VoidAir) => self.blocks += 1,
            (BlockState::Air | BlockState::CaveAir | BlockState::VoidAir, _) => self.blocks -= 1,
            (..) => (),
        }

        let id = *self.palette.entry(block).or_insert(self.next);
        if self.next == id {
            self.next += 1;
        }

        let index =
            ((position.y as usize) << 8) | ((position.z as usize) << 4) | (position.x as usize);
        self.data[index] = block.to_id();

        Some(id)
    }

    /// # Panics
    /// Will panic if (x, y, z) are >= 16
    #[must_use]
    pub fn get_block(
        &self,
        position: vek::Vec3<u8>,
    ) -> Option<BlockState> {
        if !(position.x < 16 && position.y < 16 && position.z < 16) {
            return None;
        }

        // TODO: Look into potential optimization
        let index =
            ((position.y as usize) << 8) | ((position.z as usize) << 4) | (position.x as usize);
        let id = self.data[index];

        let (block, _) = self
            .palette
            .iter()
            .find(|(_, cmp_id)| **cmp_id == id)
            .expect("Palette is missing block");
        Some(*block)
    }
}

impl<E: From<EncodeError>> Generate<E> for ChunkSection {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, E> {
        let before = buf.len();

        let mut full_palette = vec![BlockState::Air.to_id(); usize::from(self.next)];
        for (block, id) in &self.palette {
            full_palette[usize::from(*id)] = block.to_id();
        }

        _ = self.blocks.generate_in_place(buf)?;

        let bits = u8::try_from(full_palette.len().ilog2() + 1).expect("Literally impossible");
        match bits {
            0 => unreachable!(),
            1 => {
                // Single Valued
                buf.push(0u8);
                _ = encode::write_varint(i32::from(full_palette[0])).generate_in_place(buf)?;
                buf.push(0u8);
            },
            2..=8 => {
                // Palette
                let bits = 15;
                buf.push(bits);

                //_ = encode::write_varint(i32::try_from(full_palette.len()).expect("Unreachable"))
                //    .generate_in_place(buf)?;
                //for block_id in full_palette {
                //    _ = encode::write_varint(i32::from(block_id)).generate_in_place(buf)?;
                //}

                // How many longs required to store the blocks
                let blocks_per_long = 64 / bits;
                _ = encode::write_varint(4096i32.div_ceil(i32::from(blocks_per_long)))
                    .generate_in_place(buf)?;

                for index in (0..16 * 16 * 16).step_by(usize::from(blocks_per_long)) {
                    let mut value = 0u64;
                    for i in 0..blocks_per_long {
                        let idx = index + usize::from(i);
                        if let Some(data) = self.data.get(idx) {
                            value |= u64::from(*data) << (i * bits);
                        }
                    }
                    _ = value.generate_in_place(buf)?;
                }
            },
            _ => {
                // Direct
                buf.push(bits);
                todo!()
            },
        }

        // Biomes
        buf.push(0u8);
        buf.push(0u8);
        buf.push(0u8);

        Ok(buf.len() - before)
    }
}

#[derive(Debug)]
pub struct Chunk {
    pub height: Range<i32>,
    sections:   BTreeMap<i32, ChunkSection>,
}

impl Chunk {
    #[must_use]
    pub const fn new(
        min_y: i32,
        max_y: i32,
    ) -> Self {
        Self {
            height:   Range {
                start: min_y,
                end:   max_y,
            },
            sections: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn total_height(&self) -> i32 {
        self.height.end - self.height.start
    }

    /// # Panics
    /// Will panic if internal logic fails (unreachable)
    #[must_use]
    pub fn get_block(
        &mut self,
        position: vek::Vec3<i32>,
    ) -> Option<BlockState> {
        if !((0..16).contains(&position.x)
            && (0..16).contains(&position.y)
            && self.height.contains(&position.y))
        {
            return None;
        }

        // Convert height to section
        self.sections
            .entry(position.y.div_floor(16))
            .or_default()
            .get_block(
                position
                    .with_y(position.y.abs() % 16)
                    .numcast()
                    .expect("Size check failed"),
            )
    }

    /// # Panics
    /// Will panic if internal logic fails (unreachable)
    pub fn set_block(
        &mut self,
        position: vek::Vec3<i32>,
        block: BlockState,
    ) -> Option<u16> {
        if !((0..16).contains(&position.x)
            && (0..16).contains(&position.y)
            && self.height.contains(&position.y))
        {
            return None;
        }

        // Convert height to section
        self.sections
            .entry(position.y.div_floor(16))
            .or_default()
            .set_block(
                position
                    .with_y(position.y.abs() % 16)
                    .numcast()
                    .expect("Size check failed"),
                block,
            )
    }
}

impl<E: From<EncodeError> + From<crab_nbt::error::Error>> Generate<E> for &Chunk {
    fn generate_in_place(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<usize, E> {
        // TODO: Heightmap
        let heightmap = nbt!("root", {});
        let empty = ChunkSection::new();

        let mut chunk_data = Vec::new();
        for index in self.height.into_iter().step_by(16).map(|i| i / 16) {
            let section = self.sections.get(&index).unwrap_or(&empty);
            _ = <ChunkSection as Generate<E>>::generate_in_place(section, &mut chunk_data)?;
        }

        (
            heightmap,
            encode::write_varint(i32::try_from(chunk_data.len()).expect("Chunk Data too large")),
            chunk_data.as_slice(),
        )
            .generate_in_place(buf)
    }
}
