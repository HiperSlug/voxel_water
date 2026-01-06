use std::mem;

use bevy::prelude::*;

use super::block::{BLOCKS, BlockIndex};
use super::space::{AREA, Index3d, VOL, LEN};

pub struct Chunk {
    // TODO: unpad this
    pub voxels: [Option<BlockIndex>; VOL],

    pub occupied: [u64; AREA],
    pub liquid: [u64; AREA],
    pub transparent: [u64; AREA],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            voxels: [None; VOL],
            occupied: [0; AREA],
            liquid: [0; AREA],
            transparent: [0; AREA],
        }
    }
}

impl Chunk {
    pub fn set(&mut self, pos: impl Index3d, voxel: Option<BlockIndex>) {
        let i_3d = pos.i_3d();

        self.voxels[i_3d] = voxel;
        self.set_masks(pos, voxel);
    }

    fn set_masks(&mut self, pos: impl Index3d, voxel: Option<BlockIndex>) {
        let (x, i_2d) = pos.x_and_i_2d();
        let mask = 1 << x;

        let (occupied, liquid, transparent) = match voxel {
            Some(block_index) => {
                let block = &BLOCKS[block_index];
                (true, block.liquid, block.transparent)
            }
            None => (false, false, false),
        };

        let occupied_mask = if occupied { mask } else { 0 };
        let liquid_mask = if liquid { mask } else { 0 };
        let transparent_mask = if transparent { mask } else { 0 };

        self.occupied[i_2d] = (self.occupied[i_2d] & !mask) | occupied_mask;
        self.liquid[i_2d] = (self.liquid[i_2d] & !mask) | liquid_mask;
        self.transparent[i_2d] = (self.transparent[i_2d] & !mask) | transparent_mask;
    }

    pub fn transfer_within(&mut self, dst: impl Index3d, src: impl Index3d) {
        let (src_x, src_i_2d, src_i_3d) = src.x_and_i_2d_and_i_3d();
        let (dst_x, dst_i_2d, dst_i_3d) = dst.x_and_i_2d_and_i_3d();

        self.voxels[dst_i_3d] = mem::take(&mut self.voxels[src_i_3d]);

        self.set_masks((dst_x, dst_i_2d), None);

        let copy = |src: u64, dst: &mut u64| {
            *dst |= ((src >> src_x) & 1) << dst_x;
        };

        copy(self.occupied[src_i_2d], &mut self.occupied[dst_i_2d]);
        copy(self.liquid[src_i_2d], &mut self.liquid[dst_i_2d]);
        copy(self.transparent[src_i_2d], &mut self.transparent[dst_i_2d]);

        self.set_masks((src_x, src_i_2d), None);
    }

    // TODO: Rehaul for multi-chunk
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

                if self.occupied[i_2d] & (1 << x) != 0 {
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
