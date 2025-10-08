use ndshape::{ConstPow2Shape2u32, ConstShape as _};
use std::array;

pub const BITS: u32 = 6;
pub const LEN: usize = 1 << BITS; // 64
pub const AREA: usize = LEN * LEN;
// pub const VOL: usize = LEN * LEN * LEN;

pub type Shape2d = ConstPow2Shape2u32<BITS, BITS>;

pub const STRIDE_0: usize = 1 << Shape2d::SHIFTS[0];
pub const STRIDE_1: usize = 1 << Shape2d::SHIFTS[1];

pub const PAD_MASK: u64 = (1 << 63) | 1;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub some_mask: [u64; AREA],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            some_mask: [0; AREA],
        }
    }
}

impl Chunk {
    // dev fn to set inital state easily
    pub fn nz_init() -> Self {
        Self {
            some_mask: array::from_fn(|i| {
                let [y, z] = delinearize_2d(i);
                if y == 0 || y == LEN as u32 - 1 || z == 0 || z == LEN as u32 - 1 {
                    u64::MAX
                    // 0
                // } else if z == LEN as u32 / 2 {
                //     // ((1 << 16) - 1) << (16 + 8)
                //     rand::random::<u64>()
                //     & !PAD_MASK
                } else {
                    if (y % 2 == 0) ^ (z % 2 == 0) {
                        0xAAAAAAAAAAAAAAAA | PAD_MASK
                    } else {
                        0x5555555555555555 | PAD_MASK
                    }
                    // PAD_MASK
                    // rand::random::<u64>() | PAD_MASK
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
