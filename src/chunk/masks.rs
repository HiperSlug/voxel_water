use ndshape::{ConstPow2Shape2u32, ConstShape as _};

use super::*;

pub const PAD_MASK: u64 = (1 << 63) | 1;

pub type Shape2d = ConstPow2Shape2u32<BITS, BITS>;

pub const STRIDE_Y_2D: usize = 1 << Shape2d::SHIFTS[0];
pub const STRIDE_Z_2D: usize = 1 << Shape2d::SHIFTS[1];

pub trait Index2d: Copy {
    fn index_2d(self) -> usize;
}

#[derive(Clone)]
pub struct Masks {
    pub some_mask: [u64; AREA],
    pub liquid_mask: [u64; AREA],
}

impl Default for Masks {
    fn default() -> Self {
        Self {
            some_mask: [0; AREA],
            liquid_mask: [0; AREA],
        }
    }
}

impl Masks {
    pub fn set(&mut self, p: impl Index3d, v: Option<Voxel>) {
        let (i, shift) = p.index_shift_2d();
        let mask = 1 << shift;

        match v {
            Some(Voxel::Liquid) => {
                self.some_mask[i] |= mask;
                self.liquid_mask[i] |= mask;
            }
            Some(_) => {
                self.some_mask[i] |= mask;
                self.liquid_mask[i] &= !mask;
            }
            None => {
                self.some_mask[i] &= !mask;
                self.liquid_mask[i] &= !mask;
            }
        }
    }

    pub fn fill_row(&mut self, p: impl Index2d, v: Option<Voxel>) {
        let i = p.index_2d();

        match v {
            Some(Voxel::Liquid) => {
                self.some_mask[i] = u64::MAX;
                self.liquid_mask[i] = u64::MAX;
            }
            Some(_) => {
                self.some_mask[i] = u64::MAX;
                self.liquid_mask[i] = 0;
            }
            None => {
                self.some_mask[i] = 0;
                self.liquid_mask[i] = 0;
            }
        }
    }

    pub fn set_row_padding(&mut self, p: impl Index2d, v: Option<Voxel>) {
        let i = p.index_2d();

        match v {
            Some(Voxel::Liquid) => {
                self.some_mask[i] |= PAD_MASK;
                self.liquid_mask[i] |= PAD_MASK;
            }
            Some(_) => {
                self.some_mask[i] |= PAD_MASK;
                self.liquid_mask[i] &= !PAD_MASK;
            }
            None => {
                self.some_mask[i] &= !PAD_MASK;
                self.liquid_mask[i] &= !PAD_MASK;
            }
        }
    }

    pub fn is_some(&self, p: impl Index3d) -> bool {
        let (i, shift) = p.index_shift_2d();

        self.some_mask[i] & (1 << shift) != 0
    }
}

impl Index2d for usize {
    fn index_2d(self) -> usize {
        self
    }
}

impl Index2d for [u32; 2] {
    fn index_2d(self) -> usize {
        Shape2d::linearize(self) as usize
    }
}

impl Index2d for UVec2 {
    fn index_2d(self) -> usize {
        self.to_array().index_2d()
    }
}
