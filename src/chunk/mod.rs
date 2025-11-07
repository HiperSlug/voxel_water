pub mod index;
mod liquid_tick;
mod row;

use bevy::prelude::*;
use std::mem;

use index::{Index2d, Index3d};

pub use row::*;

pub const BITS: u32 = 6;

pub const LEN: usize = 1 << BITS; // 64
pub const LEN_U32: u32 = LEN as u32;
pub const AREA: usize = LEN * LEN;
pub const VOL: usize = LEN * LEN * LEN;

pub type Voxels = [Option<Voxel>; VOL];
pub type Masks = [RowMasks; AREA];

// TODO: runtime enumeration/indexing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voxel {
    Liquid,
    Solid,
}

pub struct Chunk {
    pub voxels: Voxels,
    pub masks: Masks,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            voxels: [default(); VOL],
            masks: [default(); AREA],
        }
    }
}

impl Chunk {
    pub fn transfer(&mut self, dst: impl Index3d, src: impl Index3d) {
        let src_i_3d = src.i_3d();
        let dst_i_3d = dst.i_3d();
        let (src_x, src_i_2d) = src.x_and_i_2d();
        let (dst_x, dst_i_2d) = dst.x_and_i_2d();

        let voxel = mem::take(&mut self.voxels[src_i_3d]);
        self.voxels[dst_i_3d] = voxel;

        self.masks[dst_i_2d].clear(dst_x);

        self.masks[dst_i_2d].occupied |= ((self.masks[src_i_2d].occupied >> src_x) & 1) << dst_x;
        self.masks[dst_i_2d].liquid |= ((self.masks[src_i_2d].liquid >> src_x) & 1) << dst_x;
        self.masks[dst_i_2d].opaque |= ((self.masks[src_i_2d].opaque >> src_x) & 1) << dst_x;
        self.masks[dst_i_2d].transparent |=
            ((self.masks[src_i_2d].transparent >> src_x) & 1) << dst_x;

        self.masks[src_i_2d].clear(src_x);
    }

    pub fn set(&mut self, p: impl Index3d, v: Option<Voxel>) {
        let i_3d = p.i_3d();
        let (x, i_2d) = p.x_and_i_2d();

        self.voxels[i_3d] = v;
        self.masks[i_2d].set(x, v);
    }

    pub fn fill_padding(&mut self, v: Option<Voxel>) {
        // +-Z
        for z in [0, LEN_U32 - 1] {
            for y in 0..LEN_U32 {
                let i_2d = [y, z].i_2d();
                self.masks[i_2d].fill(v);
                for x in 0..LEN_U32 {
                    let i_3d = [x, y, z].i_3d();
                    self.voxels[i_3d] = v;
                }
            }
        }

        // +-Y
        for z in 1..LEN_U32 - 1 {
            for y in [0, LEN_U32 - 1] {
                let i_2d = [y, z].i_2d();
                self.masks[i_2d].fill(v);
                for x in 0..LEN_U32 {
                    let i_3d = [x, y, z].i_3d();
                    self.voxels[i_3d] = v;
                }
            }
        }

        // +-X
        for z in 1..LEN_U32 - 1 {
            for y in 1..LEN_U32 - 1 {
                let i_2d = [y, z].i_2d();
                self.masks[i_2d].fill_padding(v);
                for x in [0, LEN_U32 - 1] {
                    let i_3d = [x, y, z].i_3d();
                    self.voxels[i_3d] = v;
                }
            }
        }
    }

    pub fn raycast(&self, ray: Ray3d, max: f32) -> [Option<UVec3>; 2] {
        let origin = ray.origin.to_vec3a();
        let dir = ray.direction.to_vec3a();

        let mut pos = origin.floor().as_ivec3();
        let step = dir.signum().as_ivec3();

        let t_delta = dir.recip().abs();
        let mut t_max = (pos.as_vec3a() + step.max(IVec3::ZERO).as_vec3a() - origin) / dir;

        let mut last = None;
        let mut distance;

        loop {
            let in_unpad_bounds =
                pos.cmpge(IVec3::ONE).all() && pos.cmplt(IVec3::splat(LEN as i32 - 1)).all();
            if in_unpad_bounds {
                let pos = pos.as_uvec3();

                let (x, i_2d) = pos.x_and_i_2d();

                if self.masks[i_2d].is_occupied(x) {
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

#[derive(Component, Deref, DerefMut, Default)]
pub struct BoxChunk(Box<Chunk>);
