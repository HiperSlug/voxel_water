use std::hint::black_box;
use std::mem::MaybeUninit;
use std::ops::Range;
use std::{array, iter};

use bevy::math::U8Vec3;
use bevy::prelude::*;
use bit_iter::BitIter;
use dashmap::DashMap;
use itertools::{ArrayCombinations, Itertools};

use crate::block::BlockIndex;
use crate::chunk::index::Index3d;
use crate::chunk::{Chunk, LEN};
use crate::render::{ChunkMesh, ChunkRemesh};

#[derive(Default, Component)]
pub struct SparseChunks {
    map: DashMap<IVec3, Chunk>,
}

#[derive(Default, Component)]
pub struct SparseChunkMeshes {
    map: DashMap<IVec3, ChunkMesh>,
}

#[derive(Default, Component)]
pub struct SparseChunkRemeshes {
    map: DashMap<IVec3, ChunkRemesh>,
}

impl SparseChunks {
    pub fn fill(&self, range: Range<VoxelPos>, voxel: Option<BlockIndex>) ->  {
        for z in range.start.z..=range.end.z {
            for y in range.start.y..=range.end.y {
                for x in range.start.x..=range.end.x {
                    let position = VoxelPos::new(x, y, z);
                    for (big, small) in position.all_chunk_pairs() {
                        self.map.entry(big).or_default().set(small, voxel);
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct VoxelPos(pub IVec3);

impl VoxelPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    pub fn main_chunk_pair(self) -> (IVec3, U8Vec3) {
        let big = self.0.div_euclid(IVec3::splat(62));
        let small = self.0.rem_euclid(IVec3::splat(62)).as_u8vec3() + U8Vec3::ONE;
        (big, small)
    }

    pub fn all_chunk_pairs(self) -> ChunkPairIter {
        ChunkPairIter::new(self)
    }
}

pub struct ChunkPairIter {
    big: IVec3,
    small: U8Vec3,
    subset_mask: u8,
    none_mask: u8,
    delta: [(u8, i8); 3],
}

impl ChunkPairIter {
    pub fn new(pos: VoxelPos) -> Self {
        let (big, small) = pos.main_chunk_pair();

        let mut none_mask = 0;
        let delta = array::from_fn(|i| {
            match small[i] {
                1 => (63, -1),
                62 => (0, 1),
                _ => {
                    none_mask |= 1 << i;
                    (!0, !0) // unused
                }
            }
        });
        
        Self {
            big,
            small,
            subset_mask: u8::MAX, // immidiately wrapped to 0
            none_mask,
            delta,
        }
    }
}

impl Iterator for ChunkPairIter {
    type Item = (IVec3, U8Vec3);

    fn next(&mut self) -> Option<Self::Item> {
        self.subset_mask = self.subset_mask.wrapping_add(1);

        if self.subset_mask == 0 {
            return Some((self.big, self.small));
        } else if self.none_mask == 0b111 {
            return None;
        }
        while self.subset_mask <= 0b111 {
            if self.subset_mask & self.none_mask == 0 {
                let mut adj_small = self.small;
                let mut adj_big = self.big;
                
                for i in BitIter::from(self.subset_mask & 0b111) {
                    let (set_small, add_big) = self.delta[i];
                    debug_assert_ne!(set_small, !0);
                    adj_small[i] = set_small;
                    adj_big[i] += add_big as i32;
                }

                return Some((adj_big, adj_small));
            }
            self.subset_mask += 1;
        }
        None
    }
}


#[cfg(test)]
mod tests {
    // AI
    use super::*;
    use std::collections::HashSet;

    fn collect(pos: VoxelPos) -> HashSet<(IVec3, U8Vec3)> {
        pos.all_chunk_pairs().collect()
    }

    #[test]
    fn interior_voxel() {
        let pos = VoxelPos(IVec3::new(10, 20, 30));
        let set = collect(pos);

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn face_voxel() {
        let pos = VoxelPos(IVec3::new(63, 10, 20));
        let set = collect(pos);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn edge_voxel() {
        let pos = VoxelPos(IVec3::new(62, 62, 10));
        let set = collect(pos);

        assert_eq!(set.len(), 4);
    }

    #[test]
    fn corner_voxel() {
        let pos = VoxelPos(IVec3::new(-1, -61, -61));
        let set = collect(pos);

        assert_eq!(set.len(), 8);
    }
}