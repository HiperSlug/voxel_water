use bevy::prelude::*;
use ndshape::{ConstPow2Shape2u32, ConstPow2Shape3u32, ConstShape as _};
use rand::{SeedableRng, rngs::SmallRng};

use crate::double_buffered::DoubleBuffered;

pub const BITS: u32 = 6;
pub const LEN: usize = 1 << BITS; // 64
pub const LEN_U32: u32 = LEN as u32;
pub const AREA: usize = LEN * LEN;
pub const VOL: usize = LEN * LEN * LEN;

type Shape2d = ConstPow2Shape2u32<BITS, BITS>;
type Shape3d = ConstPow2Shape3u32<BITS, BITS, BITS>;

// Shape2d::STRIDE_* == Shape3d::STRIDE_*
pub const STRIDE_0: usize = 1 << Shape3d::SHIFTS[0];
pub const STRIDE_1: usize = 1 << Shape3d::SHIFTS[1];
pub const STRIDE_2: usize = 1 << Shape3d::SHIFTS[2];

pub const PAD_MASK: u64 = (1 << 63) | 1;

// TODO: runtime enumeration/indexing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voxel {
    Liquid,
    Solid,
}

#[derive(Resource)]
pub struct Chunk {
    pub voxels: [Option<Voxel>; VOL],
    pub masks: DoubleBuffered<Masks>,
    pub prng: SmallRng,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            voxels: [None; VOL],
            masks: default(),
            prng: SmallRng::seed_from_u64(0),
        }
    }
}

impl Chunk {
    pub fn set(&mut self, p: impl Into<[u32; 3]> + Copy, v: Option<Voxel>) {
        let i = linearize_3d(p.into());
        self.voxels[i] = v;

        self.masks.front_mut().set(p, v);
    }

    pub fn set_padding(&mut self, v: Option<Voxel>) {
        let masks = self.masks.front_mut();

        // +-Z
        for z in [0, LEN_U32 - 1] {
            for y in 0..LEN_U32 {
                masks.set_row([y, z], v);
                for x in 0..LEN_U32 {
                    let i = linearize_3d([x, y, z]);
                    self.voxels[i] = v;
                }
            }
        }

        // +-Y
        for z in 1..LEN_U32 - 1 {
            for y in [0, LEN_U32 - 1] {
                masks.set_row([y, z], v);
                for x in 0..LEN_U32 {
                    let i = linearize_3d([x, y, z]);
                    self.voxels[i] = v;
                }
            }
        }

        // +-X
        for z in 1..LEN_U32 - 1 {
            for y in 1..LEN_U32 - 1 {
                masks.set_padding([y, z], v);
                for x in [0, LEN_U32 - 1] {
                    let i = linearize_3d([x, y, z]);
                    self.voxels[i] = v;
                }
            }
        }
    }

    /// # Panic
    /// - axial rays
    pub fn raycast(&self, ray: Ray3d, max: f32) -> Option<UVec3> {
        let masks = self.masks.front();

        let origin = ray.origin.to_vec3a();
        let dir = ray.direction.to_vec3a();

        let mut pos = origin.floor().as_ivec3();
        // dir
        let step = dir.signum().as_ivec3();

        // magnitude
        let t_delta = dir.recip().abs();
        let mut t_max = (pos.as_vec3a() + step.max(IVec3::ZERO).as_vec3a() - origin) / dir;

        let mut last = None;

        loop {
            let in_unpad_bounds =
                pos.cmpge(IVec3::ONE).all() && pos.cmplt(IVec3::splat(LEN as i32 - 1)).all();
            if in_unpad_bounds {
                let pos = pos.as_uvec3();
                if masks.is_some(pos) {
                    return Some(pos);
                }
                last = Some(pos);
            }

            if t_max.x < t_max.y && t_max.x < t_max.z {
                pos.x += step.x;
                t_max.x += t_delta.x;
                if t_max.x > max {
                    return last;
                }
            } else if t_max.y < t_max.z {
                pos.y += step.y;
                t_max.y += t_delta.y;
                if t_max.y > max {
                    return last;
                }
            } else {
                pos.z += step.z;
                t_max.z += t_delta.z;
                if t_max.z > max {
                    return last;
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct Masks {
    pub some_mask: [u64; AREA],
    pub liquid_mask: [u64; AREA],
}

impl Default for Masks {
    fn default() -> Self {
        Self {
            some_mask: [0; AREA],
            liquid_mask: [0; AREA],
        }
    }
}

impl Masks {
    pub fn set(&mut self, p: impl Into<[u32; 3]>, v: Option<Voxel>) {
        let [x, y, z] = p.into();
        let i = linearize_2d([y, z]);
        let mask = 1 << x;

        match v {
            Some(Voxel::Liquid) => {
                self.some_mask[i] |= mask;
                self.liquid_mask[i] |= mask;
            }
            Some(_) => {
                self.some_mask[i] |= mask;
                self.liquid_mask[i] &= !mask;
            }
            None => {
                self.some_mask[i] &= !mask;
                self.liquid_mask[i] &= !mask;
            }
        }
    }

    pub fn set_row(&mut self, p: impl Into<[u32; 2]>, v: Option<Voxel>) {
        let i = linearize_2d(p);

        match v {
            Some(Voxel::Liquid) => {
                self.some_mask[i] = u64::MAX;
                self.liquid_mask[i] = u64::MAX;
            }
            Some(_) => {
                self.some_mask[i] = u64::MAX;
                self.liquid_mask[i] = 0;
            }
            None => {
                self.some_mask[i] = 0;
                self.liquid_mask[i] = 0;
            }
        }
    }

    pub fn set_padding(&mut self, p: impl Into<[u32; 2]>, v: Option<Voxel>) {
        let i = linearize_2d(p);

        match v {
            Some(Voxel::Liquid) => {
                self.some_mask[i] |= PAD_MASK;
                self.liquid_mask[i] |= PAD_MASK;
            }
            Some(_) => {
                self.some_mask[i] |= PAD_MASK;
                self.liquid_mask[i] &= !PAD_MASK;
            }
            None => {
                self.some_mask[i] &= !PAD_MASK;
                self.liquid_mask[i] &= !PAD_MASK;
            }
        }
    }

    pub fn is_some(&self, p: impl Into<[u32; 3]>) -> bool {
        let [x, y, z] = p.into();
        let i = linearize_2d([y, z]);

        self.some_mask[i] & (1 << x) != 0
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
