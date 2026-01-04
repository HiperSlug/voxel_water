use bevy::prelude::*;
use ndshape::{ConstPow2Shape2u32, ConstPow2Shape3u32, ConstShape as _};

use super::BITS;

pub type Shape3d = ConstPow2Shape3u32<BITS, BITS, BITS>;
pub type Shape2d = ConstPow2Shape2u32<BITS, BITS>;

pub const STRIDE_X_3D: usize = 1 << Shape3d::SHIFTS[0];
pub const STRIDE_Y_3D: usize = 1 << Shape3d::SHIFTS[1];
pub const STRIDE_Z_3D: usize = 1 << Shape3d::SHIFTS[2];

pub const STRIDE_Y_2D: usize = 1 << Shape2d::SHIFTS[0];
pub const STRIDE_Z_2D: usize = 1 << Shape2d::SHIFTS[1];

pub const I_STRIDE_X_3D: isize = STRIDE_X_3D as isize;
pub const I_STRIDE_Y_3D: isize = STRIDE_Y_3D as isize;
pub const I_STRIDE_Z_3D: isize = STRIDE_Z_3D as isize;

pub const I_STRIDE_Y_2D: isize = STRIDE_Y_2D as isize;
pub const I_STRIDE_Z_2D: isize = STRIDE_Z_2D as isize;

const MASK_X: usize = Shape3d::MASKS[0] as usize;

pub trait Index2d: Copy {
    fn i_2d(&self) -> usize;

    fn yz(&self) -> [u32; 2];
}

pub trait Index3d: Copy {
    fn i_3d(&self) -> usize;

    fn x_and_i_2d(&self) -> (u32, usize);

    fn xyz(&self) -> [u32; 3];

    fn i_3d_and_x_and_i_2d(&self) -> (usize, u32, usize) {
        let (x, i_2d) = self.x_and_i_2d();
        (self.i_3d(), x, i_2d)
    }
}

impl Index2d for usize {
    fn i_2d(&self) -> usize {
        *self
    }

    fn yz(&self) -> [u32; 2] {
        Shape2d::delinearize(*self as u32)
    }
}

impl Index2d for [u32; 2] {
    fn i_2d(&self) -> usize {
        Shape2d::linearize(*self) as usize
    }

    fn yz(&self) -> [u32; 2] {
        *self
    }
}

impl Index2d for UVec2 {
    fn i_2d(&self) -> usize {
        self.to_array().i_2d()
    }

    fn yz(&self) -> [u32; 2] {
        self.to_array()
    }
}

impl Index3d for usize {
    fn i_3d(&self) -> usize {
        *self
    }

    fn x_and_i_2d(&self) -> (u32, usize) {
        ((*self & MASK_X) as u32, *self >> BITS)
    }

    fn xyz(&self) -> [u32; 3] {
        Shape3d::delinearize(*self as u32)
    }
}

impl Index3d for (usize, usize) {
    fn i_3d(&self) -> usize {
        let (x, i_2d) = *self;
        (i_2d << BITS) | x
    }

    fn x_and_i_2d(&self) -> (u32, usize) {
        (self.0 as u32, self.1)
    }

    fn xyz(&self) -> [u32; 3] {
        let (x, i_2d) = *self;
        let [y, z] = i_2d.yz();
        [x as u32, y, z]
    }
}

impl Index3d for (u32, usize) {
    fn i_3d(&self) -> usize {
        let (x, i_2d) = *self;
        (i_2d << BITS) | x as usize
    }

    fn x_and_i_2d(&self) -> (u32, usize) {
        *self
    }

    fn xyz(&self) -> [u32; 3] {
        let (x, i_2d) = *self;
        let [y, z] = i_2d.yz();
        [x, y, z]
    }
}

impl Index3d for [u32; 3] {
    fn i_3d(&self) -> usize {
        Shape3d::linearize(*self) as usize
    }

    fn x_and_i_2d(&self) -> (u32, usize) {
        let [x, y, z] = *self;
        (x, [y, z].i_2d())
    }

    fn xyz(&self) -> [u32; 3] {
        *self
    }
}

impl Index3d for UVec3 {
    fn i_3d(&self) -> usize {
        self.to_array().i_3d()
    }

    fn x_and_i_2d(&self) -> (u32, usize) {
        self.to_array().x_and_i_2d()
    }

    fn xyz(&self) -> [u32; 3] {
        self.to_array()
    }
}
