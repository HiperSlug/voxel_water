use bevy::prelude::*;
use enum_map::{Enum, EnumMap, enum_map};
use std::{
    cell::RefCell,
    f32::consts::{FRAC_PI_2, PI},
};

use crate::chunk::{
    AREA, Chunk, LEN, LEN_U32, PAD_MASK, STRIDE_0, STRIDE_1, STRIDE_2, Voxel, linearize_2d,
    linearize_3d,
};

use Face::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Enum)]
pub enum Face {
    PosX,
    PosY,
    PosZ,
    NegX,
    NegY,
    NegZ,
}

impl Face {
    const ALL: [Self; 6] = [PosX, PosY, PosZ, NegX, NegY, NegZ];
}

#[derive(Debug, Clone, Copy)]
pub struct Quad {
    pub pos: UVec3,
    pub size: UVec2,
    pub face: Face,
    // TODO: runtime texture indexing
    pub voxel: Voxel,
}

impl Quad {
    pub fn rectangle_transform(&self) -> Transform {
        let pos = self.pos.as_vec3();
        let scale = self.size.as_vec2().extend(1.0);
        let half_size = self.size.as_vec2() / 2.0;

        match self.face {
            PosX => Transform {
                translation: pos + vec3(1.0, half_size.y, half_size.x),
                rotation: Quat::from_rotation_y(FRAC_PI_2),
                scale,
            },
            NegX => Transform {
                translation: pos + vec3(0.0, half_size.y, half_size.x),
                rotation: Quat::from_rotation_y(-FRAC_PI_2),
                scale,
            },
            PosY => Transform {
                translation: pos + vec3(half_size.x, 1.0, half_size.y),
                rotation: Quat::from_rotation_x(-FRAC_PI_2),
                scale,
            },
            NegY => Transform {
                translation: pos + vec3(half_size.x, 0.0, half_size.y),
                rotation: Quat::from_rotation_x(FRAC_PI_2),
                scale,
            },
            PosZ => Transform {
                translation: pos + vec3(half_size.x, half_size.y, 1.0),
                rotation: Quat::default(),
                scale,
            },
            NegZ => Transform {
                translation: pos + vec3(half_size.x, half_size.y, 0.0),
                rotation: Quat::from_rotation_y(-PI),
                scale,
            },
        }
    }
}

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
    fn clear(&mut self) {
        self.quads.clear();
        // TODO: also may not need to be zeroed
        self.upward_merged.fill(0);
        // TODO: ditto
        self.forward_merged.fill(0);
        // TODO: ditto
        for (_, arr) in &mut *self.visible_masks {
            arr.fill(0)
        }
    }

    fn build_visible_masks(&mut self, chunk: &Chunk) {
        const STRIDE_Y: usize = STRIDE_0;
        const STRIDE_Z: usize = STRIDE_1;

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
                        PosY => some_mask[i + STRIDE_Y],
                        NegY => some_mask[i - STRIDE_Y],
                        PosZ => some_mask[i + STRIDE_Z],
                        NegZ => some_mask[i - STRIDE_Z],
                    };

                    visible_mask[i] = unpad_some & !adj_some;
                }
            }
        }
    }

    fn face_merging(&mut self, chunk: &Chunk) {
        const STRIDE_X_3D: usize = STRIDE_0;
        const STRIDE_Y_3D: usize = STRIDE_1;
        const STRIDE_Z_3D: usize = STRIDE_2;

        const STRIDE_Y_2D: usize = STRIDE_0;
        const STRIDE_Z_2D: usize = STRIDE_1;

        const FORWARD_STRIDE_X: usize = STRIDE_0;
        const FORWARD_STRIDE_Y: usize = STRIDE_1;

        const UPWARD_STRIDE_X: usize = STRIDE_0;

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

                                    let pos = uvec3(x, y, z);
                                    let size = uvec2(w, h);
                                    Quad {
                                        pos,
                                        size,
                                        face,
                                        voxel,
                                    }
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

                                    let pos = uvec3(x, y, z);
                                    let size = uvec2(w, h);

                                    Quad {
                                        pos,
                                        size,
                                        face,
                                        voxel,
                                    }
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

                                    let pos = uvec3(x, y, z);
                                    let size = uvec2(w, h);

                                    Quad {
                                        pos,
                                        size,
                                        face,
                                        voxel,
                                    }
                                });

                                self.upward_merged[upward_i] = 0;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn mesh(&mut self, chunk: &Chunk) -> &[Quad] {
        self.clear();
        self.build_visible_masks(chunk);
        self.face_merging(chunk);
        &self.quads
    }
}
