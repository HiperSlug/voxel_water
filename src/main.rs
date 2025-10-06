use bevy::{math::{IVec3, Quat, UVec2, Vec3}, transform::components::Transform};
use enum_map::{enum_map, Enum, EnumMap};
use ndshape::{ConstPow2Shape3u32, ConstShape as _};
use std::iter;

fn main() {
    println!("Hello, world!");
}

const BITS: u32 = 6;
const LEN: usize = 1 << BITS; // 64
const AREA: usize = LEN * LEN;
const VOL: usize = LEN * LEN * LEN;
type Shape = ConstPow2Shape3u32<BITS, BITS, BITS>;
const UNPAD_MASK: u64 = (1 << 63) | 1;

#[derive(Clone)]
struct Chunk {
    some_masks: [u64; AREA],
}

pub use Face::*;

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
    pub const ALL: [Self; 6] = [PosX, PosY, PosZ, NegX, NegY, NegZ];
}

struct Quad {
    transform: Transform
}

pub struct Mesher {
    quads: EnumMap<Face, Vec<Quad>>,
    visible_masks: Box<EnumMap<Face, [u64; AREA]>>,
    upward_merged: Box<[u8; LEN]>,
    forward_merged: Box<[u8; AREA]>,
}

impl Mesher {
    pub fn new() -> Self {
        Self {
            quads: enum_map! { _ => Vec::new() },
            visible_masks: Box::new(enum_map! { _ => [0; AREA] }),
            upward_merged: Box::new([0; LEN]),
            forward_merged: Box::new([0; AREA]),
        }
    }

    pub fn clear(&mut self) {
        self.quads.clear();
        self.visible_masks.as_mut_array().fill([0; AREA]);
        self.upward_merged.fill(0);
        self.forward_merged.fill(0);
    }

    fn face_culling(&mut self, some_masks: &[u64; AREA]) {
        for face in Face::ALL {
            for z in 1..LEN - 1 {
                for y in 1..LEN - 1 {
                    let i = Shape::linearize([y, z, 0]);

                    let some_mask = some_masks[i];
                    let unpad_some_mask = some_mask & UNPAD_MASK;

                    if unpad_some_mask == 0 {
                        continue;
                    }

                    let adj_some_mask = match face {
                        PosX => some_mask >> 1,
                        NegX => some_mask << 1,
                        PosY => some_masks[i + (1 << Shape::SHIFTS[0])],
                        NegY => some_masks[i - (1 << Shape::SHIFTS[0])],
                        PosZ => some_masks[i + (1 << Shape::SHIFTS[1])],
                        NegZ => some_masks[i - (1 << Shape::SHIFTS[1])],
                    };

                    self.visible_masks[face][area_yz] = unpad_some_mask & !adj_some_mask;
                }
            }
        }
    }

    fn face_merging(&mut self) {
        let mut offsets = [0; 7];

        for (index, face) in [PosX, PosY, PosZ, NegX, NegY, NegZ].into_iter().enumerate() {
            let visible_mask = &self.visible_masks[face];

            for z in 1..LEN - 1 {
                let vol_z = z << SHIFT_2;

                let area_z = z << SHIFT_1;

                for y in 1..LEN - 1 {
                    let vol_y = y << SHIFT_1;
                    let vol_yz = vol_y | vol_z;

                    let area_y = y << SHIFT_0;
                    let area_yz = area_y | area_z;

                    let mut column = visible_mask[area_yz];
                    if column == 0 {
                        continue;
                    }

                    match face {
                        PosX | NegX => {
                            let upward_column = visible_mask[area_yz + STRIDE_0];

                            let forward_column = visible_mask[area_yz + STRIDE_1];

                            while column != 0 {
                                let x = column.trailing_zeros() as usize;
                                column &= column - 1;

                                let vol_x = x << SHIFT_0;
                                let vol_xyz = vol_x | vol_yz;

                                let vol_xy = vol_x | vol_y;

                                let voxel_opt = voxel_opts[vol_xyz];
                                let voxel = voxel_opt.unwrap();

                                if self.upward_merged[vol_x] == 0
                                    && (forward_column >> x) & 1 != 0
                                    && voxel_opt == voxel_opts[vol_xyz + STRIDE_2]
                                {
                                    self.forward_merged[vol_xy] += 1;
                                    continue;
                                }

                                if (upward_column >> x) & 1 != 0
                                    && self.forward_merged[vol_xy]
                                        == self.forward_merged[vol_xy + STRIDE_1]
                                    && voxel_opt == voxel_opts[vol_xyz + STRIDE_1]
                                {
                                    self.forward_merged[vol_xy] = 0;
                                    self.upward_merged[vol_x] += 1;
                                    continue;
                                }

                                let w = self.forward_merged[vol_xy] as u32;
                                let h = self.upward_merged[vol_x] as u32 + 1;

                                let x = x as i32;
                                let y = y as i32 - self.upward_merged[vol_x] as i32;
                                let z = z as i32 - self.forward_merged[vol_xy] as i32;

                                self.forward_merged[vol_xy] = 0;
                                self.upward_merged[vol_x] = 0;

                                let pos = chunk_origin + IVec3::new(x, y, z);
                                let texture_index = voxel.textures()[face];

                                let quad = VoxelQuad::new(pos, texture_index, w, h, face);
                                self.quads.push(quad);
                            }
                        }
                        PosY | NegY => {
                            let forward_column = visible_mask[area_yz + STRIDE_1];

                            while column != 0 {
                                let x = column.trailing_zeros() as usize;

                                let vol_x = x << SHIFT_0;
                                let vol_xyz = vol_x | vol_yz;

                                let vol_xy = vol_x | vol_y;

                                let voxel_opt = voxel_opts[vol_xyz];
                                let voxel = voxel_opt.unwrap();

                                if (forward_column >> x) & 1 != 0
                                    && voxel_opt == voxel_opts[vol_xyz + STRIDE_2]
                                {
                                    self.forward_merged[vol_xy] += 1;
                                    column &= column - 1;
                                    continue;
                                }

                                let mut right_merged = 1;
                                for right in (x + 1)..LEN - 1 {
                                    let r_vol_x = right << SHIFT_0;
                                    let r_vol_xy = r_vol_x | vol_y;

                                    if (column >> right) & 1 == 0
                                        || self.forward_merged[vol_xy]
                                            != self.forward_merged[r_vol_xy]
                                        || voxel_opt != voxel_opts[r_vol_xy | vol_z]
                                    {
                                        break;
                                    }
                                    self.forward_merged[r_vol_xy] = 0;
                                    right_merged += 1;
                                }
                                let cleared = x + right_merged;
                                column &= !((1 << cleared) - 1);

                                let w = right_merged as u32;
                                let h = self.forward_merged[vol_xy] as u32 + 1;

                                let x = x as i32;
                                let y = y as i32;
                                let z = z as i32 - self.forward_merged[vol_xy] as i32;

                                self.forward_merged[vol_xy] = 0;

                                let pos = chunk_origin + IVec3::new(x, y, z);
                                let texture_index = voxel.textures()[face];

                                let quad = VoxelQuad::new(pos, texture_index, w, h, face);
                                self.quads.push(quad);
                            }
                        }
                        PosZ | NegZ => {
                            let upward_column = visible_mask[area_yz + STRIDE_0];

                            while column != 0 {
                                let x = column.trailing_zeros() as usize;

                                let vol_x = x << SHIFT_0;
                                let vol_xyz = vol_x | vol_yz;

                                let voxel_opt = voxel_opts[vol_xyz];
                                let voxel = voxel_opt.unwrap();

                                if (upward_column >> x) & 1 != 0
                                    && voxel_opt == voxel_opts[vol_xyz + STRIDE_1]
                                {
                                    self.upward_merged[vol_x] += 1;
                                    column &= column - 1;
                                    continue;
                                }

                                let mut right_merged = 1;
                                for right in (x + 1)..LEN - 1 {
                                    if (column >> right) & 1 == 0
                                        || self.upward_merged[vol_x] != self.upward_merged[right]
                                        || voxel_opt != {
                                            let vol_x = right << SHIFT_0;
                                            let vol_xyz = vol_x | vol_yz;
                                            voxel_opts[vol_xyz]
                                        }
                                    {
                                        break;
                                    }
                                    self.upward_merged[right] = 0;
                                    right_merged += 1;
                                }
                                let cleared = x + right_merged;
                                column &= !((1 << cleared) - 1);

                                let w = right_merged as u32;
                                let h = self.upward_merged[vol_x] as u32 + 1;

                                let x = x as i32;
                                let y = y as i32 - self.upward_merged[vol_x] as i32;
                                let z = z as i32;

                                self.upward_merged[vol_x] = 0;

                                let pos = chunk_origin + IVec3::new(x, y, z);
                                let texture_index = voxel.textures()[face];

                                let quad = VoxelQuad::new(pos, texture_index, w, h, face);
                                self.quads.push(quad);
                            }
                        }
                    }
                }
            }
            offsets[index + 1] = self.quads.len() as u32;
        }

        VoxelQuadOffsets(offsets)
    }

    pub fn mesh(&mut self, chunk: &Chunk, chunk_pos: IVec3) -> (&[VoxelQuad], VoxelQuadOffsets) {
        let Chunk {
            voxel_opts,
            opaque_mask,
            transparent_mask,
        } = chunk;

        let chunk_origin = chunk_origin(chunk_pos);

        self.face_culling(voxel_opts, opaque_mask, transparent_mask);

        let voxel_quad_offsets = self.face_merging(voxel_opts, chunk_origin);

        (&self.quads, voxel_quad_offsets)
    }
}
