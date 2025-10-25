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

    fn yz(&self) -> [u32; 2];
}

impl Index2d for usize {
    #[inline]
    fn i_2d(&self) -> usize {
        *self
    }

    #[inline]
    fn yz(&self) -> [u32; 2] {
        Shape2d::delinearize(*self as u32)
    }
}

impl Index2d for [u32; 2] {
    #[inline]
    fn i_2d(&self) -> usize {
        Shape2d::linearize(*self) as usize
    }

    #[inline]
    fn yz(&self) -> [u32; 2] {
        *self
    }
}

impl Index2d for UVec2 {
    #[inline]
    fn i_2d(&self) -> usize {
        self.to_array().i_2d()
    }

    #[inline]
    fn yz(&self) -> [u32; 2] {
        self.to_array()
    }
}

pub trait Index3d {
    fn i_3d(&self) -> usize;

    fn x_and_i_2d(&self) -> (u32, usize);

    fn xyz(&self) -> [u32; 3];
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

    #[inline]
    fn xyz(&self) -> [u32; 3] {
        Shape3d::delinearize(*self as u32)
    }
}

impl Index3d for (u32, usize) {
    #[inline]
    fn i_3d(&self) -> usize {
        let (x, i_2d) = *self;
        (i_2d << BITS) | x as usize
    }

    #[inline]
    fn x_and_i_2d(&self) -> (u32, usize) {
        *self
    }

    #[inline]
    fn xyz(&self) -> [u32; 3] {
        let (x, i_2d) = *self;
        let [y, z] = i_2d.yz();
        [x, y, z]
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

    #[inline]
    fn xyz(&self) -> [u32; 3] {
        *self
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

    #[inline]
    fn xyz(&self) -> [u32; 3] {
        self.to_array()
    }
}
