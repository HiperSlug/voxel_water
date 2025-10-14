use bevy::prelude::*;
use enum_map::{EnumMap, enum_map};
use std::cell::RefCell;

use super::*;

use crate::chunk::{
    AREA, Chunk, LEN, LEN_U32, PAD_MASK, STRIDE_X_3D, STRIDE_Y_2D, STRIDE_Y_3D, STRIDE_Z_2D,
    STRIDE_Z_3D, Voxel, linearize_2d, linearize_3d,
};

const UPWARD_STRIDE_X: usize = STRIDE_X_3D;
const FORWARD_STRIDE_X: usize = STRIDE_X_3D;
const FORWARD_STRIDE_Y: usize = STRIDE_Y_3D;

thread_local! {
    pub static MESHER: RefCell<Mesher> = default();
}

// TODO: iterative meshing
// TODO: remove scratch `quads`
// TODO: transparency
#[derive(Debug)]
pub struct Mesher {
    quads: Vec<Quad>,
    visible_masks: Box<EnumMap<Face, [u64; AREA]>>,
    upward_merged: Box<[u8; LEN]>,
    forward_merged: Box<[u8; AREA]>,
}

impl Default for Mesher {
    fn default() -> Self {
        Self {
            quads: Vec::new(),
            visible_masks: Box::new(enum_map! { _ => [0; AREA] }),
            upward_merged: Box::new([0; LEN]),
            forward_merged: Box::new([0; AREA]),
        }
    }
}

impl Mesher {
    // TODO: determine nececcity, after transparency
    fn clear(&mut self) {
        self.quads.clear();
        self.upward_merged.fill(0);
        self.forward_merged.fill(0);
        for (_, arr) in &mut *self.visible_masks {
            arr.fill(0)
        }
    }

    fn build_visible_masks(&mut self, chunk: &Chunk) {
        let some_mask = &chunk.masks.front().some_mask;

        for face in Face::ALL {
            let visible_mask = &mut self.visible_masks[face];

            for z in 1..LEN_U32 - 1 {
                for y in 1..LEN_U32 - 1 {
                    let i = linearize_2d([y, z]);

                    let some = some_mask[i];
                    let unpad_some = some & !PAD_MASK;

                    if unpad_some == 0 {
                        continue;
                    }

                    let adj_some = match face {
                        PosX => some >> 1,
                        NegX => some << 1,
                        PosY => some_mask[i + STRIDE_Y_2D],
                        NegY => some_mask[i - STRIDE_Y_2D],
                        PosZ => some_mask[i + STRIDE_Z_2D],
                        NegZ => some_mask[i - STRIDE_Z_2D],
                    };

                    visible_mask[i] = unpad_some & !adj_some;
                }
            }
        }
    }

    fn face_merging(&mut self, chunk: &Chunk, origin: IVec3) {
        for face in Face::ALL {
            let visible_mask = &mut self.visible_masks[face];
            for z in 1..LEN_U32 - 1 {
                for y in 1..LEN_U32 - 1 {
                    let i = linearize_2d([y, z]);

                    let mut visible = visible_mask[i];
                    if visible == 0 {
                        continue;
                    }

                    match face {
                        PosX | NegX => {
                            let upward_visible = visible_mask[i + STRIDE_Y_2D];
                            let forward_visible = visible_mask[i + STRIDE_Z_2D];

                            while visible != 0 {
                                let x = visible.trailing_zeros();
                                visible &= visible - 1;

                                let upward_i = x as usize;
                                let forward_i = linearize_2d([x, y]);

                                let i = linearize_3d([x, y, z]);
                                let voxel_opt = chunk.voxels[i];
                                let voxel = voxel_opt.unwrap();

                                // forward merging
                                if self.upward_merged[upward_i] == 0
                                    && (forward_visible >> x) & 1 != 0
                                    && voxel_opt == chunk.voxels[i + STRIDE_Z_3D]
                                {
                                    self.forward_merged[forward_i] += 1;
                                    continue;
                                }

                                // upward merging
                                if (upward_visible >> x) & 1 != 0
                                    && self.forward_merged[forward_i]
                                        == self.forward_merged[forward_i + FORWARD_STRIDE_Y]
                                    && voxel_opt != chunk.voxels[i + STRIDE_Y_3D]
                                {
                                    self.forward_merged[forward_i] = 0;
                                    self.upward_merged[upward_i] += 1;
                                    continue;
                                }

                                // finish
                                self.quads.push({
                                    let forward_merged = self.forward_merged[forward_i] as u32;
                                    let upward_merged = self.upward_merged[upward_i] as u32;

                                    let w = forward_merged + 1;
                                    let h = upward_merged + 1;

                                    let y = y - upward_merged;
                                    let z = z - forward_merged;

                                    let pos = uvec3(x, y, z).as_ivec3() + origin;

                                    // TODO: change placeholder
                                    let t = match voxel {
                                        Voxel::Liquid => 0,
                                        Voxel::Solid => 1,
                                    };

                                    Quad::new(pos, w, h, face, t)
                                });

                                self.forward_merged[forward_i] = 0;
                                self.upward_merged[upward_i] = 0;
                            }
                        }
                        PosY | NegY => {
                            let forward_visible = visible_mask[i + STRIDE_Z_2D];

                            while visible != 0 {
                                let x = visible.trailing_zeros();

                                let forward_i = linearize_2d([x, y]);

                                let i = linearize_3d([x, y, z]);
                                let voxel_opt = chunk.voxels[i];
                                let voxel = voxel_opt.unwrap();

                                // forward merging
                                if (forward_visible >> x) & 1 != 0
                                    && voxel_opt == chunk.voxels[i + STRIDE_Y_3D]
                                {
                                    self.forward_merged[forward_i] += 1;
                                    visible &= visible - 1;
                                    continue;
                                }

                                // rightward merging
                                let mut right_merged = 1;
                                let mut forward_next_i = forward_i;
                                let mut next_i_3d = i;
                                for x in x + 1..LEN_U32 - 1 {
                                    forward_next_i += FORWARD_STRIDE_X;
                                    next_i_3d += STRIDE_X_3D;

                                    if (visible >> x) & 1 == 0
                                        || self.forward_merged[forward_i]
                                            != self.forward_merged[forward_next_i]
                                        || voxel_opt != chunk.voxels[next_i_3d]
                                    {
                                        break;
                                    }
                                    self.forward_merged[forward_next_i] = 0;
                                    right_merged += 1;
                                }
                                let cleared = x + right_merged;
                                visible &= !((1 << cleared) - 1);

                                // finish
                                self.quads.push({
                                    let forward_merged = self.forward_merged[forward_i] as u32;

                                    let w = right_merged;
                                    let h = forward_merged + 1;

                                    let z = z - forward_merged;

                                    let pos = uvec3(x, y, z).as_ivec3() + origin;

                                    let t = match voxel {
                                        Voxel::Liquid => 0,
                                        Voxel::Solid => 1,
                                    };

                                    Quad::new(pos, w, h, face, t)
                                });

                                self.forward_merged[forward_i] = 0
                            }
                        }
                        PosZ | NegZ => {
                            let upward_visible = visible_mask[i + STRIDE_Y_2D];

                            while visible != 0 {
                                let x = visible.trailing_zeros();

                                let upward_i = x as usize;

                                let i = linearize_3d([x, y, z]);
                                let voxel_opt = chunk.voxels[i];
                                let voxel = voxel_opt.unwrap();

                                // upward merging
                                if (upward_visible >> x) & 1 != 0
                                    && voxel_opt == chunk.voxels[i + STRIDE_Y_3D]
                                {
                                    self.upward_merged[upward_i] += 1;
                                    visible &= visible - 1;
                                    continue;
                                }

                                // rightward merging
                                let mut right_merged = 1;
                                let mut upward_next_i = upward_i;
                                let mut next_i_3d = i;
                                for x in x + 1..LEN_U32 - 1 {
                                    upward_next_i += UPWARD_STRIDE_X;
                                    next_i_3d += STRIDE_X_3D;

                                    if (visible >> x) & 1 == 0
                                        || self.upward_merged[upward_i]
                                            != self.upward_merged[upward_next_i]
                                        || voxel_opt != chunk.voxels[next_i_3d]
                                    {
                                        break;
                                    }
                                    self.upward_merged[upward_next_i] = 0;
                                    right_merged += 1;
                                }
                                let cleared = x + right_merged;
                                visible &= !((1 << cleared) - 1);

                                // finish
                                self.quads.push({
                                    let upward_merged = self.upward_merged[upward_i] as u32;

                                    let w = right_merged;
                                    let h = upward_merged + 1;

                                    let y = y - upward_merged;

                                    let pos = uvec3(x, y, z).as_ivec3() + origin;

                                    let t = match voxel {
                                        Voxel::Liquid => 0,
                                        Voxel::Solid => 1,
                                    };

                                    Quad::new(pos, w, h, face, t)
                                });

                                self.upward_merged[upward_i] = 0;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn mesh(&mut self, chunk: &Chunk, chunk_pos: IVec3) -> &[Quad] {
        let origin = chunk_pos * LEN as i32;
        self.clear();
        self.build_visible_masks(chunk);
        self.face_merging(chunk, origin);
        &self.quads
    }
}
