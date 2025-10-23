pub mod double_buffered;
mod liquid_tick;
mod masks;

use bevy::{platform::collections::HashMap, prelude::*};
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

pub const MASK_X: usize = Shape3d::MASKS[0] as usize;

pub type Voxels = [Option<Voxel>; VOL];

// TODO: runtime enumeration/indexing
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voxel {
    Liquid,
    Solid,
}

pub struct DoubleBufferedChunk {
    pub voxels: Voxels,
    pub masks: DoubleBuffered<Masks>,
}

impl Default for DoubleBufferedChunk {
    fn default() -> Self {
        Self {
            voxels: [None; VOL],
            masks: default(),
        }
    }
}

impl DoubleBufferedChunk {
    pub fn front_mut(&mut self) -> FrontMut {
        FrontMut {
            voxels: &mut self.voxels,
            masks: self.masks.front_mut(),
        }
    }

    pub fn swap_sync_mut(&mut self) -> (FrontMut, &mut Masks) {
        let [front, back] = self.masks.swap_mut();
        *front = back.clone();
        (
            FrontMut {
                voxels: &mut self.voxels,
                masks: front,
            },
            back,
        )
    }
}

#[derive(Default)]
pub struct Chunk {
    pub double_buffered_chunk: DoubleBufferedChunk,
    pub dst_to_src: HashMap<usize, usize>,
}

pub struct FrontMut<'a> {
    pub voxels: &'a mut Voxels,
    pub masks: &'a mut Masks,
}

impl<'a> FrontMut<'a> {
    pub fn set(&mut self, p: impl Index3d, v: Option<Voxel>) {
        self.voxels[p.index_3d()] = v;
        self.masks.set(p, v);
    }

    pub fn fill_padding(&mut self, v: Option<Voxel>) {
        // +-Z
        for z in [0, LEN_U32 - 1] {
            for y in 0..LEN_U32 {
                self.masks.fill_row([y, z], v);
                for x in 0..LEN_U32 {
                    let i = [x, y, z].index_3d();
                    self.voxels[i] = v;
                }
            }
        }

        // +-Y
        for z in 1..LEN_U32 - 1 {
            for y in [0, LEN_U32 - 1] {
                self.masks.fill_row([y, z], v);
                for x in 0..LEN_U32 {
                    let i = [x, y, z].index_3d();
                    self.voxels[i] = v;
                }
            }
        }

        // +-X
        for z in 1..LEN_U32 - 1 {
            for y in 1..LEN_U32 - 1 {
                self.masks.set_row_padding([y, z], v);
                for x in [0, LEN_U32 - 1] {
                    let i = [x, y, z].index_3d();
                    self.voxels[i] = v;
                }
            }
        }
    }

    pub fn raycast(&self, ray: Ray3d, max: f32) -> [Option<UVec3>; 2] {
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

pub trait Index3d: Copy {
    fn index_3d(self) -> usize;

    fn index_shift_2d(self) -> (usize, usize);
}

impl Index3d for usize {
    fn index_3d(self) -> usize {
        self
    }

    fn index_shift_2d(self) -> (usize, usize) {
        (self >> BITS, self & MASK_X)
    }
}

impl Index3d for [u32; 3] {
    fn index_3d(self) -> usize {
        Shape3d::linearize(self) as usize
    }

    fn index_shift_2d(self) -> (usize, usize) {
        let [x, y, z] = self;
        ([y, z].index_2d(), x as usize)
    }
}

impl Index3d for UVec3 {
    fn index_3d(self) -> usize {
        self.to_array().index_3d()
    }

    fn index_shift_2d(self) -> (usize, usize) {
        self.to_array().index_shift_2d()
    }
}
