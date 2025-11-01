mod double_buffered;
pub mod index;
mod liquid_tick;
pub mod masks;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use index::Index3d;
use masks::Masks;

pub use masks::PAD_MASK;

pub const BITS: u32 = 6;

pub const LEN: usize = 1 << BITS; // 64
pub const LEN_U32: u32 = LEN as u32;
pub const AREA: usize = LEN * LEN;
pub const VOL: usize = LEN * LEN * LEN;

pub type Mask = [u64; AREA];
pub type Voxels = [Option<Voxel>; VOL];

pub const DEFAULT_MASK: Mask = [0; AREA];
pub const DEFAULT_VOXELS: Voxels = [None; VOL];

// TODO: runtime enumeration/indexing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voxel {
    Liquid,
    Solid,
}

pub struct Chunk {
    pub voxels: Voxels,
    pub masks: Masks,
    pub dst_to_src: HashMap<usize, usize>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            voxels: DEFAULT_VOXELS,
            masks: default(),
            dst_to_src: default(),
        }
    }
}

impl Chunk {
    pub fn set(&mut self, p: impl Index3d, v: Option<Voxel>) {
        self.voxels[p.i_3d()] = v;

        self.masks.set(p, v);
    }

    pub fn fill_padding(&mut self, v: Option<Voxel>) {
        // +-Z
        for z in [0, LEN_U32 - 1] {
            for y in 0..LEN_U32 {
                self.masks.fill_row([y, z], v);
                for x in 0..LEN_U32 {
                    let i = [x, y, z].i_3d();
                    self.voxels[i] = v;
                }
            }
        }

        // +-Y
        for z in 1..LEN_U32 - 1 {
            for y in [0, LEN_U32 - 1] {
                self.masks.fill_row([y, z], v);
                for x in 0..LEN_U32 {
                    let i = [x, y, z].i_3d();
                    self.voxels[i] = v;
                }
            }
        }

        // +-X
        for z in 1..LEN_U32 - 1 {
            for y in 1..LEN_U32 - 1 {
                self.masks.set_row_padding([y, z], v);
                for x in [0, LEN_U32 - 1] {
                    let i = [x, y, z].i_3d();
                    self.voxels[i] = v;
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

                if self.masks.is_some(pos) {
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
