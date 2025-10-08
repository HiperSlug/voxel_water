// TODO: don't remove these comments even if they are ugly
// TODO: research if BVH is a good idea

use bevy::prelude::*;
use ndshape::{ConstPow2Shape2u32, ConstPow2Shape3u32, ConstShape as _};

pub const BITS: u32 = 6;
pub const LEN: usize = 1 << BITS; // 64
pub const LEN_U32: u32 = LEN as u32;
pub const AREA: usize = LEN * LEN;
pub const VOL: usize = LEN * LEN * LEN;

type Shape2d = ConstPow2Shape2u32<BITS, BITS>;
type Shape3d = ConstPow2Shape3u32<BITS, BITS, BITS>;

// STRIDE_* for Shape3d == Shape2d
pub const STRIDE_0: usize = 1 << Shape3d::SHIFTS[0];
pub const STRIDE_1: usize = 1 << Shape3d::SHIFTS[1];
pub const STRIDE_2: usize = 1 << Shape3d::SHIFTS[2];

pub const PAD_MASK: u64 = (1 << 63) | 1;

// TODO: runtime enumeration/indexing
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voxel {
    Liquid, 
    Solid, 
}

#[derive(Clone)]
pub struct Chunk {
    pub voxels: [Option<Voxel>; VOL],
    pub some_mask: [u64; AREA],
    pub liquid_mask: [u64; AREA],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            voxels: [default(); VOL],
            some_mask: [default(); AREA],
            liquid_mask: [default(); AREA],
        }
    }
}

impl Chunk {
    #[inline(never)]
    pub fn set(&mut self, p: impl Into<[u32; 3]>, v: Option<Voxel>) {
        let [x, y, z] = p.into();
        let i_2d = linearize_2d([y, z]);
        let i_3d = linearize_3d([x, y, z]);
        
        self.voxels[i_3d] = v;

        let x_mask = 1 << x;
        
        if let Some(voxel) = v {
            self.some_mask[i_2d] |= x_mask;
            
            if let Voxel::Liquid = voxel {
                self.liquid_mask[i_2d] |= x_mask;
            } else {
                self.liquid_mask[i_2d] &= !x_mask;
            }
        } else {
            self.some_mask[i_2d] &= !x_mask;
        }
    }

    pub fn set_padding(&mut self, v: Option<Voxel>) {
        // +-Z
        for z in [0, LEN_U32 - 1] {
            for y in 0..LEN_U32 {
                for x in 0..LEN_U32 {
                    self.set([x, y, z], v);
                }
            }
        }

        // +-Y
        for z in 1..LEN_U32 - 1 {
            for y in [0, LEN_U32 - 1] {
                for x in 0..LEN_U32 {
                    self.set([x, y, z], v);
                }
            }
        }

        // +-X
        for z in 1..LEN_U32 - 1 {
            for y in 1..LEN_U32 - 1 {
                for x in [0, LEN_U32 - 1] {
                    self.set([x, y, z], v);
                }
            }
        }
    }

    /// # Source
    /// https://github.com/splashdust/bevy_voxel_world/blob/main/src/voxel_traversal.rs#L93 \
    /// && http://www.cse.yorku.ca/~amana/research/grid.pdf
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

#[inline]
pub fn linearize_3d(p: impl Into<[u32; 3]>) -> usize {
    Shape3d::linearize(p.into()) as usize
}

#[inline]
pub fn delinearize_3d(i: usize) -> [u32; 3] {
    Shape3d::delinearize(i as u32)
}