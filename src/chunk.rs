use bevy::prelude::*;
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

// BVH
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
                    // } else if z >= 16 || z < 48 {
                    // (((1 << 32) - 1) << 16)
                    // rand::random::<u64>()
                    // | PAD_MASK
                } else {
                    // if (y % 2 == 0) ^ (z % 2 == 0) {
                    //     0xAAAAAAAAAAAAAAAA | PAD_MASK
                    // } else {
                    //     0x5555555555555555 | PAD_MASK
                    // }
                    PAD_MASK
                    // rand::random::<u64>() | PAD_MASK
                }
            }),
        }
    }

    pub fn set(&mut self, pos: UVec3, to: bool) {
        let i = linearize_2d([pos.y, pos.z]);
        self.some_mask[i] &= !(1 << pos.x);
        self.some_mask[i] |= (to as u64) << pos.x;
    }

    pub fn raycast(&self, ray: Ray3d, max: f32) -> Option<UVec3> {
        let mut voxel = ray.origin.floor().as_ivec3();

        let step = ray.direction.signum().as_ivec3();

        let t_delta = ray.direction.abs().recip();
        let mut t_max =
            (voxel.as_vec3() + step.max(IVec3::ZERO).as_vec3() - ray.origin) / *ray.direction;

        let mut last = None;

        loop {
            if voxel.cmpge(IVec3::ONE).all() && voxel.cmplt(IVec3::splat(LEN as i32 - 1)).all() {
                let i = linearize_2d([voxel.y as u32, voxel.z as u32]);
                if (self.some_mask[i] >> voxel.x) & 1 != 0 {
                    return Some(voxel.as_uvec3());
                }
                last = Some(voxel.as_uvec3());
            }

            if t_max.x < t_max.y && t_max.x < t_max.z {
                voxel.x += step.x;
                t_max.x += t_delta.x;
                if t_max.x.abs() > max {
                    return last;
                }
            } else if t_max.y < t_max.z {
                voxel.y += step.y;
                t_max.y += t_delta.y;
                if t_max.y.abs() > max {
                    return last;
                }
            } else {
                voxel.z += step.z;
                t_max.z += t_delta.z;
                if t_max.z.abs() > max {
                    return last;
                }
            }
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
