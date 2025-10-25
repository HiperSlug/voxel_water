use bevy::prelude::*;
use ndshape::{ConstPow2Shape2u32, ConstPow2Shape3u32, ConstShape as _};

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
    #[inline]
    fn i_2d(&self) -> usize {
        *self
    }
}

impl Index2d for [u32; 2] {
    #[inline]
    fn i_2d(&self) -> usize {
        Shape2d::linearize(*self) as usize
    }
}

impl Index2d for UVec2 {
    #[inline]
    fn i_2d(&self) -> usize {
        self.to_array().i_2d()
    }
}

pub trait Index3d {
    fn i_3d(&self) -> usize;

    fn x_and_i_2d(&self) -> (u32, usize);
}

impl Index3d for usize {
    #[inline]
    fn i_3d(&self) -> usize {
        *self
    }

    #[inline]
    fn x_and_i_2d(&self) -> (u32, usize) {
        ((*self & MASK_X) as u32, *self >> BITS)
    }
}

impl Index3d for (u32, usize) {
    #[inline]
    fn i_3d(&self) -> usize {
        let (shift, i_2d) = *self;
        (i_2d << BITS) | shift as usize
    }

    #[inline]
    fn x_and_i_2d(&self) -> (u32, usize) {
        *self
    }
}

impl Index3d for [u32; 3] {
    #[inline]
    fn i_3d(&self) -> usize {
        Shape3d::linearize(*self) as usize
    }

    #[inline]
    fn x_and_i_2d(&self) -> (u32, usize) {
        let [x, y, z] = *self;
        (x, [y, z].i_2d())
    }
}

impl Index3d for UVec3 {
    #[inline]
    fn i_3d(&self) -> usize {
        self.to_array().i_3d()
    }

    #[inline]
    fn x_and_i_2d(&self) -> (u32, usize) {
        self.to_array().x_and_i_2d()
    }
}
