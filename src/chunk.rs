pub mod pad {
    pub const BITS: u32 = 6;
    pub const LEN: usize = 1 << BITS; // 64
    pub const AREA: usize = LEN * LEN;
    // pub const VOL: usize = LEN * LEN * LEN;
}

pub mod unpad {
    use super::pad;

    pub const LEN: usize = pad::LEN - 2; // 62
    // pub const AREA: usize = LEN * LEN;
    // pub const VOL: usize = LEN * LEN * LEN;
}

use bevy::prelude::*;
use ndshape::{ConstPow2Shape2u32, ConstShape as _};
use rand::random;
use std::array;

use pad::{AREA, BITS, LEN};

pub type Shape2d = ConstPow2Shape2u32<BITS, BITS>;

pub const SHIFT_0: u32 = Shape2d::SHIFTS[0];
pub const SHIFT_1: u32 = Shape2d::SHIFTS[1];

pub const STRIDE_0: usize = 1 << SHIFT_0;
pub const STRIDE_1: usize = 1 << SHIFT_1;

pub const PAD_MASK: u64 = (1 << 63) | 1;

#[derive(Debug, Resource)]
pub struct Chunk {
    pub some_mask: [u64; AREA],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            some_mask: array::from_fn(|i| {
                let [y, z] = delinearize_2d(i);
                if y == 0 || y == LEN as u32 - 1 || z == 0 || z == LEN as u32 - 1 {
                    0
                } else {
                    // random::<u64>() & !PAD_MASK
                    !PAD_MASK
                }
            }),
        }
    }
}

#[inline]
pub fn linearize_2d(p: impl Into<[u32; 2]>) -> usize {
    Shape2d::linearize(p.into()) as usize
}

#[inline]
pub fn delinearize_2d(i: usize) -> [u32; 2] {
    Shape2d::delinearize(i as u32)
}
