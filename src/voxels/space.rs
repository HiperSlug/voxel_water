mod chunk_pair_iter;
mod index;

use bevy::math::U8Vec3;
use bevy::prelude::*;
pub use chunk_pair_iter::*;
pub use index::*;

pub const BITS: u32 = 6;

pub const LEN: usize = 1 << BITS; // 64
pub const LEN_U32: u32 = LEN as u32;
pub const AREA: usize = LEN * LEN;
pub const VOL: usize = LEN * LEN * LEN;

pub const UNPAD_LEN: usize = LEN - 2;
pub const UNPAD_LEN_I32: i32 = UNPAD_LEN as i32;
pub const UNPAD_VOL: usize = UNPAD_LEN * UNPAD_LEN * UNPAD_LEN;

pub const PAD_MASK: u64 = (1 << 63) | 1;

#[derive(Clone, Copy, Deref, DerefMut, PartialEq, Eq, Default, Debug, Hash)]
pub struct GlobalPos(pub IVec3);

#[derive(Clone, Copy, Deref, DerefMut, PartialEq, Eq, Default, Debug, Hash)]
pub struct ChunkBigPos(pub IVec3);

#[derive(Clone, Copy, Deref, DerefMut, PartialEq, Eq, Default, Debug, Hash)]
pub struct ChunkSmallPos(pub U8Vec3);

impl GlobalPos {
    pub fn main_chunk_pair(self) -> (ChunkBigPos, ChunkSmallPos) {
        let big = self.0.div_euclid(IVec3::splat(UNPAD_LEN_I32));
        let small = self.0.rem_euclid(IVec3::splat(UNPAD_LEN_I32)).as_u8vec3() + U8Vec3::ONE;
        (ChunkBigPos(big), ChunkSmallPos(small))
    }

    pub fn chunk_pair_iter(self) -> ChunkPairIter {
        ChunkPairIter::new(self)
    }
}
