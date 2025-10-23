use bevy::prelude::*;
use ndshape::{ConstPow2Shape3u32, ConstPow2Shape2u32, ConstShape as _};

use super::*;

pub type Shape3d = ConstPow2Shape3u32<BITS, BITS, BITS>;
pub type Shape2d = ConstPow2Shape2u32<BITS, BITS>;

pub const STRIDE_X_3D: usize = 1 << Shape3d::SHIFTS[0];
pub const STRIDE_Y_3D: usize = 1 << Shape3d::SHIFTS[1];
pub const STRIDE_Z_3D: usize = 1 << Shape3d::SHIFTS[2];

pub const STRIDE_Y_2D: usize = 1 << Shape2d::SHIFTS[0];
pub const STRIDE_Z_2D: usize = 1 << Shape2d::SHIFTS[1];

const MASK_X: usize = Shape3d::MASKS[0] as usize;

pub trait Index2d {
    fn i_2d(&self) -> usize;
}

impl Index2d for usize {
    fn i_2d(&self) -> usize {
        *self
    }
}

impl Index2d for [u32; 2] {
    fn i_2d(&self) -> usize {
        Shape2d::linearize(*self) as usize
    }
}

impl Index2d for UVec2 {
    fn i_2d(&self) -> usize {
        self.to_array().i_2d()
    }
}

pub trait Index3d {
    fn i_3d(&self) -> usize;

    fn i_2d_and_shift(&self) -> [usize; 2];
}

impl Index3d for usize {
    fn i_3d(&self) -> usize {
        *self
    }

    fn i_2d_and_shift(&self) -> [usize; 2] {
        [*self >> BITS, *self & MASK_X]
    }
}

impl Index3d for [usize; 2] {
    fn i_3d(&self) -> usize {
        let [i_2d, shift] = *self;
        (i_2d << BITS) | shift
    }

    fn i_2d_and_shift(&self) -> [usize; 2] {
        *self
    }
}

impl Index3d for [u32; 3] {
    fn i_3d(&self) -> usize {
        Shape3d::linearize(*self) as usize
    }

    fn i_2d_and_shift(&self) -> [usize; 2] {
        let [x, y, z] = *self;
        [[y, z].i_2d(), x as usize]
    }
}

impl Index3d for UVec3 {
    fn i_3d(&self) -> usize {
        self.to_array().i_3d()
    }

    fn i_2d_and_shift(&self) -> [usize; 2] {
        self.to_array().i_2d_and_shift()
    }
}
