use bevy::prelude::*;
use itertools::Either;
use ndshape::{ConstPow2Shape2u32, ConstPow2Shape3u32, ConstShape as _};
use rand::random;

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

    pub fn liquid_tick(&mut self) {
        const STRIDE_Y_2D: isize = STRIDE_0 as isize;
        const STRIDE_Z_2D: isize = STRIDE_1 as isize;

        const STRIDE_X_3D: isize = STRIDE_0 as isize;
        const STRIDE_Y_3D: isize = STRIDE_1 as isize;
        const STRIDE_Z_3D: isize = STRIDE_2 as isize;

        let [read, write] = self.masks.swap_mut();

        // TODO: determine if this makes a difference
        let range = if random() {
            Either::Left(1..LEN_U32 - 1)
        } else {
            Either::Right((1..LEN_U32 - 1).rev())
        };

        for z in range.clone() {
            'row: for y in range.clone() {
                let i = linearize_2d([y, z]);
                let yz_base = linearize_3d([0, y, z]);

                let pad_liquid = read.some_mask[i];
                let mut liquid = pad_liquid & !PAD_MASK;

                if liquid == 0 {
                    continue 'row;
                }

                // down
                let ny_i = (i as isize - STRIDE_Y_2D) as usize;
                let fall = liquid & !read.some_mask[ny_i] & !write.some_mask[ny_i];
                liquid &= !fall;
                write.some_mask[ny_i] |= fall;
                write.liquid_mask[ny_i] |= fall;

                push_2d_to_3d::<{ -STRIDE_Y_3D }>(&mut self.voxels, fall, yz_base);

                if liquid == 0 {
                    continue 'row;
                }

                let ny_pz_i = (ny_i as isize + STRIDE_Z_2D) as usize;
                let ny_nz_i = (ny_i as isize - STRIDE_Z_2D) as usize;

                // random priorities
                let x_mask = random::<u64>();
                let pos_mask = random::<u64>();

                let group_masks = [
                    x_mask & pos_mask,
                    x_mask & !pos_mask,
                    !x_mask & pos_mask,
                    !x_mask & !pos_mask,
                ];

                use Dir::*;
                #[derive(Clone, Copy)]
                enum Dir {
                    PosX,
                    NegX,
                    PosZ,
                    NegZ,
                }

                const DIRS: [Dir; 4] = [PosX, NegX, PosZ, NegZ];

                // adjacent
                for j in 0..4 {
                    for k in 0..4 {
                        let dir = DIRS[k];
                        let group_mask = group_masks[(j + k) % 4];

                        let group = liquid & group_mask;
                        if group == 0 {
                            continue;
                        }

                        match dir {
                            PosX => {
                                let fall =
                                    (group >> 1) & !read.some_mask[ny_i] & !write.some_mask[ny_i];
                                liquid &= !(fall << 1);
                                write.some_mask[ny_i] |= fall;
                                write.liquid_mask[ny_i] |= fall;

                                push_2d_to_3d::<{ STRIDE_X_3D - STRIDE_Y_3D }>(
                                    &mut self.voxels,
                                    fall << 1,
                                    yz_base,
                                );
                            }
                            NegX => {
                                let fall =
                                    (group << 1) & !read.some_mask[ny_i] & !write.some_mask[ny_i];
                                liquid &= !(fall >> 1);
                                write.some_mask[ny_i] |= fall;
                                write.liquid_mask[ny_i] |= fall;

                                push_2d_to_3d::<{ -STRIDE_X_3D - STRIDE_Y_3D }>(
                                    &mut self.voxels,
                                    fall >> 1,
                                    yz_base,
                                );
                            }
                            PosZ => {
                                let fall =
                                    group & !read.some_mask[ny_pz_i] & !write.some_mask[ny_pz_i];
                                liquid &= !fall;
                                write.some_mask[ny_pz_i] |= fall;
                                write.liquid_mask[ny_pz_i] |= fall;

                                push_2d_to_3d::<{ -STRIDE_Y_3D + STRIDE_Z_3D }>(
                                    &mut self.voxels,
                                    fall,
                                    yz_base,
                                );
                            }
                            NegZ => {
                                let fall =
                                    group & !read.some_mask[ny_nz_i] & !write.some_mask[ny_nz_i];
                                liquid &= !fall;
                                write.some_mask[ny_nz_i] |= fall;
                                write.liquid_mask[ny_nz_i] |= fall;

                                push_2d_to_3d::<{ -STRIDE_Y_3D - STRIDE_Z_3D }>(
                                    &mut self.voxels,
                                    fall,
                                    yz_base,
                                );
                            }
                        }
                        if liquid == 0 {
                            continue 'row;
                        }
                    }
                }

                // diagonal
                for j in 0..4 {
                    for k in 0..4 {
                        let dir = DIRS[k];
                        let group_mask = group_masks[(j + k) % 4];

                        let group = liquid & group_mask;
                        if group == 0 {
                            continue;
                        }

                        match dir {
                            PosX => {
                                let fall = (group >> 1)
                                    & !read.some_mask[ny_nz_i]
                                    & !write.some_mask[ny_nz_i];
                                liquid &= !(fall << 1);
                                write.some_mask[ny_nz_i] |= fall;
                                write.liquid_mask[ny_nz_i] |= fall;

                                push_2d_to_3d::<{ STRIDE_X_3D - STRIDE_Y_3D - STRIDE_Z_3D }>(
                                    &mut self.voxels,
                                    fall << 1,
                                    yz_base,
                                );
                            }
                            NegX => {
                                let fall = (group << 1)
                                    & !read.some_mask[ny_pz_i]
                                    & !write.some_mask[ny_pz_i];
                                liquid &= !(fall >> 1);
                                write.some_mask[ny_pz_i] |= fall;
                                write.liquid_mask[ny_pz_i] |= fall;

                                push_2d_to_3d::<{ -STRIDE_X_3D - STRIDE_Y_3D + STRIDE_Z_3D }>(
                                    &mut self.voxels,
                                    fall >> 1,
                                    yz_base,
                                );
                            }
                            PosZ => {
                                let fall = (group >> 1)
                                    & !read.some_mask[ny_pz_i]
                                    & !write.some_mask[ny_pz_i];
                                liquid &= !(fall << 1);
                                write.some_mask[ny_pz_i] |= fall;
                                write.liquid_mask[ny_pz_i] |= fall;

                                push_2d_to_3d::<{ STRIDE_X_3D - STRIDE_Y_3D + STRIDE_Z_3D }>(
                                    &mut self.voxels,
                                    fall << 1,
                                    yz_base,
                                );
                            }
                            NegZ => {
                                let fall = (group << 1)
                                    & !read.some_mask[ny_nz_i]
                                    & !write.some_mask[ny_nz_i];
                                liquid &= !(fall >> 1);
                                write.some_mask[ny_nz_i] |= fall;
                                write.liquid_mask[ny_nz_i] |= fall;

                                push_2d_to_3d::<{ -STRIDE_X_3D - STRIDE_Y_3D - STRIDE_Z_3D }>(
                                    &mut self.voxels,
                                    fall >> 1,
                                    yz_base,
                                );
                            }
                        }
                        if liquid == 0 {
                            continue 'row;
                        }
                    }
                }

                let pz_i = (i as isize + STRIDE_Z_2D) as usize;
                let nz_i = (i as isize - STRIDE_Z_2D) as usize;

                // lateral
                for j in 0..4 {
                    for k in 0..4 {
                        let dir = DIRS[k];
                        let group_mask = group_masks[(j + k) % 4];

                        let group = liquid & group_mask;
                        if group == 0 {
                            continue;
                        }

                        match dir {
                            PosX => {
                                let slide = group
                                    & !(read.some_mask[i] << 1)
                                    & (read.some_mask[i] >> 1)
                                    & !(write.some_mask[i] << 1);
                                liquid &= !slide;
                                write.some_mask[i] |= slide >> 1;
                                write.liquid_mask[i] |= slide >> 1;

                                push_2d_to_3d::<STRIDE_X_3D>(&mut self.voxels, slide, yz_base)
                            }
                            NegX => {
                                let slide = group
                                    & !(read.some_mask[i] >> 1)
                                    & (read.some_mask[i] << 1)
                                    & !(write.some_mask[i] >> 1);
                                liquid &= !slide;
                                write.some_mask[i] |= slide << 1;
                                write.liquid_mask[i] |= slide << 1;

                                push_2d_to_3d::<{ -STRIDE_X_3D }>(&mut self.voxels, slide, yz_base)
                            }
                            PosZ => {
                                let slide = group
                                    & !read.some_mask[pz_i]
                                    & read.some_mask[nz_i]
                                    & !write.some_mask[pz_i];
                                liquid &= !slide;
                                write.some_mask[pz_i] |= slide;
                                write.liquid_mask[pz_i] |= slide;

                                push_2d_to_3d::<STRIDE_Z_3D>(&mut self.voxels, slide, yz_base)
                            }
                            NegZ => {
                                let slide = group
                                    & !read.some_mask[nz_i]
                                    & read.some_mask[pz_i]
                                    & !write.some_mask[nz_i];
                                liquid &= !slide;
                                write.some_mask[nz_i] |= slide;
                                write.liquid_mask[nz_i] |= slide;

                                push_2d_to_3d::<{ -STRIDE_Z_3D }>(&mut self.voxels, slide, yz_base)
                            }
                        }
                        if liquid == 0 {
                            continue 'row;
                        }
                    }
                }
            }
        }

        // while we're in a single chunk I manully preserve padding
        // in a multi-chunk simulation other chunks would write to eachother
        for z in [0, LEN_U32 - 1] {
            for y in 0..LEN_U32 {
                let i = linearize_2d([y, z]);

                write.some_mask[i] |= read.some_mask[i]
            }
        }

        for z in 1..LEN_U32 - 1 {
            for y in [0, LEN_U32 - 1] {
                let i = linearize_2d([y, z]);

                write.some_mask[i] |= read.some_mask[i]
            }
        }

        // zero the old read buffer
        *read = default();
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

// #[inline]
// pub fn delinearize_2d(i: usize) -> [u32; 2] {
//     Shape2d::delinearize(i as u32)
// }

#[inline]
pub fn linearize_3d(p: impl Into<[u32; 3]>) -> usize {
    Shape3d::linearize(p.into()) as usize
}

// #[inline]
// pub fn delinearize_3d(i: usize) -> [u32; 3] {
//     Shape3d::delinearize(i as u32)
// }

#[inline]
fn push_2d_to_3d<const STRIDE_3D: isize>(
    voxels: &mut [Option<Voxel>; VOL],
    mut mask: u64,
    yz_base: usize,
) {
    while mask != 0 {
        let x = mask.trailing_zeros() as usize;
        mask &= mask - 1;

        let from = yz_base | x;
        let to = (from as isize + STRIDE_3D) as usize;
        voxels[to] = voxels[from];
        voxels[from] = None;
    }
}
