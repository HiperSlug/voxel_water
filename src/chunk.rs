mod double_buffered;
mod liquid_tick;
mod masks;

use bevy::prelude::*;
use double_buffered::DoubleBuffered;
use ndshape::{ConstPow2Shape3u32, ConstShape as _};

pub use masks::*;

pub const BITS: u32 = 6;
pub const LEN: usize = 1 << BITS; // 64
pub const LEN_U32: u32 = LEN as u32;
pub const AREA: usize = LEN * LEN;
pub const VOL: usize = LEN * LEN * LEN;

pub type Shape3d = ConstPow2Shape3u32<BITS, BITS, BITS>;

pub const STRIDE_X_3D: usize = 1 << Shape3d::SHIFTS[0];
pub const STRIDE_Y_3D: usize = 1 << Shape3d::SHIFTS[1];
pub const STRIDE_Z_3D: usize = 1 << Shape3d::SHIFTS[2];

pub type Voxels = [Option<Voxel>; VOL];

// TODO: runtime enumeration/indexing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voxel {
    Liquid,
    Solid,
}

#[derive(Component)]
pub struct Chunk {
    pub voxels: Voxels,
    pub masks: DoubleBuffered<Masks>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            voxels: [None; VOL],
            masks: default(),
        }
    }
}

impl Chunk {
    pub fn set(&mut self, p: impl Into<[u32; 3]> + Copy, v: Option<Voxel>) {
        let i = linearize_3d(p);
        self.voxels[i] = v;

        self.masks.front_mut().set(p, v);
    }

    pub fn fill_padding(&mut self, v: Option<Voxel>) {
        let masks = self.masks.front_mut();

        // +-Z
        for z in [0, LEN_U32 - 1] {
            for y in 0..LEN_U32 {
                masks.fill_row([y, z], v);
                for x in 0..LEN_U32 {
                    let i = linearize_3d([x, y, z]);
                    self.voxels[i] = v;
                }
            }
        }

        // +-Y
        for z in 1..LEN_U32 - 1 {
            for y in [0, LEN_U32 - 1] {
                masks.fill_row([y, z], v);
                for x in 0..LEN_U32 {
                    let i = linearize_3d([x, y, z]);
                    self.voxels[i] = v;
                }
            }
        }

        // +-X
        for z in 1..LEN_U32 - 1 {
            for y in 1..LEN_U32 - 1 {
                masks.set_row_padding([y, z], v);
                for x in [0, LEN_U32 - 1] {
                    let i = linearize_3d([x, y, z]);
                    self.voxels[i] = v;
                }
            }
        }
    }

    pub fn raycast(&self, ray: Ray3d, max: f32) -> [Option<UVec3>; 2] {
        let masks = self.masks.front();

        let origin = ray.origin.to_vec3a();
        let dir = ray.direction.to_vec3a();

        let mut pos = origin.floor().as_ivec3();
        // direction
        let step = dir.signum().as_ivec3();

        // magnitude
        let t_delta = dir.recip().abs();
        let mut t_max = (pos.as_vec3a() + step.max(IVec3::ZERO).as_vec3a() - origin) / dir;

        let mut last = None;
        let mut distance;

        loop {
            let in_unpad_bounds =
                pos.cmpge(IVec3::ONE).all() && pos.cmplt(IVec3::splat(LEN as i32 - 1)).all();
            if in_unpad_bounds {
                let pos = pos.as_uvec3();

                if masks.is_some(pos) {
                    return [last, Some(pos)];
                }

                last = Some(pos);
            }

            if t_max.x < t_max.y && t_max.x < t_max.z {
                pos.x += step.x;
                distance = t_max.x;
                t_max.x = distance + t_delta.x;
            } else if t_max.y < t_max.z {
                pos.y += step.y;
                distance = t_max.y;
                t_max.y = distance + t_delta.y;
            } else {
                pos.z += step.z;
                distance = t_max.z;
                t_max.z = distance + t_delta.z;
            }

            if distance > max {
                return [last, None];
            }
        }
    }
}

#[inline]
pub fn linearize_3d(p: impl Into<[u32; 3]>) -> usize {
    Shape3d::linearize(p.into()) as usize
}

// #[inline]
// pub fn delinearize_3d(i: usize) -> [u32; 3] {
//     Shape3d::delinearize(i as u32)
// }
