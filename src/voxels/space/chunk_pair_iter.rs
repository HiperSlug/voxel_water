use std::array;
use std::mem::MaybeUninit;

use bit_iter::BitIter;

use super::{ChunkBigPos, ChunkSmallPos, GlobalPos};

pub struct ChunkPairIter {
    main_chunk_pair: (ChunkBigPos, ChunkSmallPos),
    subset_mask: u8,
    none_mask: u8,
    deltas: [MaybeUninit<(u8, i8)>; 3],
}

impl ChunkPairIter {
    pub fn new(pos: GlobalPos) -> Self {
        let (big, small) = pos.main_chunk_pair();

        let mut none_mask = 0;
        let deltas = array::from_fn(|i| match small[i] {
            1 => MaybeUninit::new((63, -1)),
            62 => MaybeUninit::new((0, 1)),
            _ => {
                none_mask |= 1 << i;
                MaybeUninit::uninit()
            }
        });

        Self {
            main_chunk_pair: (big, small),
            subset_mask: 0,
            none_mask,
            deltas,
        }
    }
}

impl Iterator for ChunkPairIter {
    type Item = (ChunkBigPos, ChunkSmallPos);

    fn next(&mut self) -> Option<Self::Item> {
        if self.subset_mask == 0 {
            self.subset_mask += 1;
            return Some(self.main_chunk_pair);
        } else if self.none_mask == 0b111 {
            return None;
        }

        while self.subset_mask <= 0b111 {
            if self.subset_mask & self.none_mask == 0 {
                let (mut adj_big, mut adj_small) = self.main_chunk_pair;

                for i in BitIter::from(self.subset_mask & 0b111) {
                    // SAFETY: `none_mask` tracks uninitalized values and is compared with `subset_mask`
                    let (set_small, add_big) = unsafe { self.deltas[i].assume_init() };
                    adj_small[i] = set_small;
                    adj_big[i] += add_big as i32;
                }

                self.subset_mask += 1;
                return Some((adj_big, adj_small));
            }
            self.subset_mask += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bevy::prelude::*;

    use super::*;

    fn collect(pos: GlobalPos) -> HashSet<(ChunkBigPos, ChunkSmallPos)> {
        pos.chunk_pair_iter().collect()
    }

    #[test]
    fn interior_voxel() {
        let pos = GlobalPos(IVec3::new(10, 20, 30));
        let set = collect(pos);

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn face_voxel() {
        let pos = GlobalPos(IVec3::new(63, 10, 20));
        let set = collect(pos);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn edge_voxel() {
        let pos = GlobalPos(IVec3::new(62, 62, 10));
        let set = collect(pos);

        assert_eq!(set.len(), 4);
    }

    #[test]
    fn corner_voxel() {
        let pos = GlobalPos(IVec3::new(-1, -61, -61));
        let set = collect(pos);

        assert_eq!(set.len(), 8);
    }
}
