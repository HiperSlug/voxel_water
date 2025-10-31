// TODO: transparency

use bevy::prelude::*;
use enum_map::{EnumMap, enum_map};
use std::cell::RefCell;
use std::ops::Range;

use super::*;

use crate::chunk::{AREA, Chunk, LEN, LEN_U32, PAD_MASK, Voxel, index::*};

const UPWARD_STRIDE_X: usize = STRIDE_X_3D;
const FORWARD_STRIDE_X: usize = STRIDE_X_3D;
const FORWARD_STRIDE_Y: usize = STRIDE_Y_3D;

thread_local! {
    pub static MESHER: RefCell<Mesher> = default();
}

/// Reusable buffers for meshing
#[derive(Deref, DerefMut)]
pub struct Mesher {
    quads: Vec<Quad>,
    #[deref]
    inner: InnerMesher,
}

impl Default for Mesher {
    fn default() -> Self {
        Self {
            quads: Vec::new(),
            inner: InnerMesher {
                visible_masks: Box::new(enum_map! { _ => [0; AREA] }),
                upward_merged: Box::new([0; LEN]),
                forward_merged: Box::new([0; AREA]),
            },
        }
    }
}

pub struct InnerMesher {
    visible_masks: Box<EnumMap<Face, [u64; AREA]>>,
    upward_merged: Box<[u8; LEN]>,
    forward_merged: Box<[u8; AREA]>,
}

impl InnerMesher {
    fn build_all_visible_masks(&mut self, chunk: &Chunk) {
        let some_mask = &chunk.front_masks.some_mask;

        for f in Face::ALL {
            let visible_mask = &mut self.visible_masks[f];

            for z in 1..LEN_U32 - 1 {
                for y in 1..LEN_U32 - 1 {
                    let i_2d = [y, z].i_2d();

                    let some = some_mask[i_2d];
                    let unpad_some = some & !PAD_MASK;

                    if unpad_some == 0 {
                        visible_mask[i_2d] = 0;
                    } else {
                        let adj_some = match f {
                            PosX => some >> 1,
                            NegX => some << 1,
                            PosY => some_mask[i_2d + STRIDE_Y_2D],
                            NegY => some_mask[i_2d - STRIDE_Y_2D],
                            PosZ => some_mask[i_2d + STRIDE_Z_2D],
                            NegZ => some_mask[i_2d - STRIDE_Z_2D],
                        };

                        visible_mask[i_2d] = unpad_some & !adj_some;
                    }
                }
            }
        }
    }

    fn build_visible_masks(&mut self, chunk: &Chunk, remesh: U64Vec3) {
        let some_mask = &chunk.front_masks.some_mask;

        let other_z = !remesh.z & !PAD_MASK;
        let other_y = !remesh.y & !PAD_MASK;

        for f in Face::ALL {
            let visible_mask = &mut self.visible_masks[f];

            let mut handler = |y: u32, z: u32, xs: u64| {
                let i_2d = [y, z].i_2d();

                let some = some_mask[i_2d];
                let unpad_some = some & !PAD_MASK & xs;

                if unpad_some == 0 {
                    visible_mask[i_2d] = 0;
                } else {
                    let adj_some = match f {
                        PosX => some >> 1,
                        NegX => some << 1,
                        PosY => some_mask[i_2d + STRIDE_Y_2D],
                        NegY => some_mask[i_2d - STRIDE_Y_2D],
                        PosZ => some_mask[i_2d + STRIDE_Z_2D],
                        NegZ => some_mask[i_2d - STRIDE_Z_2D],
                    };

                    visible_mask[i_2d] = unpad_some & !adj_some;
                }
            };

            for z in BitIter::from(remesh.z).map(u32) {
                for y in 1..LEN_U32 - 1 {
                    handler(y, z, !0)
                }
            }

            for z in BitIter::from(other_z).map(u32) {
                for y in BitIter::from(remesh.y).map(u32) {
                    handler(y, z, !0)
                }
            }

            for z in BitIter::from(other_z).map(u32) {
                for y in BitIter::from(other_y).map(u32) {
                    handler(y, z, remesh.x)
                }
            }
        }
    }

    fn merged_quads(&mut self, chunk: &Chunk, origin: IVec3) -> EnumMap<Face, Vec<Quad>> {
        let mut map = EnumMap::default();
        for f in Face::ALL {
            let quads = &mut map[f];
            match f {
                PosX | NegX => self.merge_x(chunk, origin, !0, f, quads),
                PosY | NegY => self.merge_y(chunk, origin, 1..LEN_U32 - 1, f, quads),
                PosZ | NegZ => self.merge_z(chunk, origin, 1..LEN_U32 - 1, f, quads),
            }
        }
        map
    }

    fn merge_x(&mut self, chunk: &Chunk, origin: IVec3, xs: u64, f: Face, quads: &mut Vec<Quad>) {
        let visible_mask = &self.visible_masks[f];

        for z in 1..LEN_U32 - 1 {
            for y in 1..LEN_U32 - 1 {
                let i_2d = [y, z].i_2d();
                let mut visible = visible_mask[i_2d] & xs;

                let upward_visible = visible_mask[i_2d + STRIDE_Y_2D] & xs;
                let forward_visible = visible_mask[i_2d + STRIDE_Z_2D] & xs;

                while visible != 0 {
                    let x = visible.trailing_zeros();
                    visible &= visible - 1;

                    let upward_i = x as usize;
                    let forward_i = [x, y].i_2d();

                    let i_3d = (x, i_2d).i_3d();
                    let voxel_opt = chunk.voxels[i_3d];
                    let voxel = voxel_opt.unwrap();

                    // forward merging
                    if self.upward_merged[upward_i] == 0
                        && (forward_visible >> x) & 1 != 0
                        && voxel_opt == chunk.voxels[i_3d + STRIDE_Z_3D]
                    {
                        self.forward_merged[forward_i] += 1;
                        continue;
                    }

                    // upward merging
                    if (upward_visible >> x) & 1 != 0
                        && self.forward_merged[forward_i]
                            == self.forward_merged[forward_i + FORWARD_STRIDE_Y]
                        && voxel_opt != chunk.voxels[i_3d + STRIDE_Y_3D]
                    {
                        self.forward_merged[forward_i] = 0;
                        self.upward_merged[upward_i] += 1;
                        continue;
                    }

                    // finish
                    quads.push({
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

                        Quad::new(pos, w, h, f, t)
                    });

                    self.forward_merged[forward_i] = 0;
                    self.upward_merged[upward_i] = 0;
                }
            }
        }

        quads.sort_unstable_by_key(|q| q.pos.x);
    }

    fn merge_y(
        &mut self,
        chunk: &Chunk,
        origin: IVec3,
        ys: impl Iterator<Item = u32> + Clone,
        f: Face,
        quads: &mut Vec<Quad>,
    ) {
        let visible_mask = &mut self.visible_masks[f];

        for z in 1..LEN_U32 - 1 {
            for y in ys.clone() {
                let i_2d = [y, z].i_2d();
                let mut visible = visible_mask[i_2d];

                let forward_visible = visible_mask[i_2d + STRIDE_Z_2D];

                while visible != 0 {
                    let x = visible.trailing_zeros();

                    let forward_i = [x, y].i_2d();

                    let i_3d = (x, i_2d).i_3d();
                    let voxel_opt = chunk.voxels[i_3d];
                    let voxel = voxel_opt.unwrap();

                    // forward merging
                    if (forward_visible >> x) & 1 != 0
                        && voxel_opt == chunk.voxels[i_3d + STRIDE_Y_3D]
                    {
                        self.forward_merged[forward_i] += 1;
                        visible &= visible - 1;
                        continue;
                    }

                    // rightward merging
                    let mut right_merged = 1;
                    let mut forward_next_i = forward_i;
                    let mut next_i_3d = i_3d;
                    for x in x + 1..LEN_U32 - 1 {
                        forward_next_i += FORWARD_STRIDE_X;
                        next_i_3d += STRIDE_X_3D;

                        if (visible >> x) & 1 == 0
                            || self.forward_merged[forward_i] != self.forward_merged[forward_next_i]
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
                    quads.push({
                        let forward_merged = self.forward_merged[forward_i] as u32;

                        let w = right_merged;
                        let h = forward_merged + 1;

                        let z = z - forward_merged;

                        let pos = uvec3(x, y, z).as_ivec3() + origin;

                        let t = match voxel {
                            Voxel::Liquid => 0,
                            Voxel::Solid => 1,
                        };

                        Quad::new(pos, w, h, f, t)
                    });

                    self.forward_merged[forward_i] = 0
                }
            }
        }

        quads.sort_unstable_by_key(|q| q.pos.y);
    }

    fn merge_z(
        &mut self,
        chunk: &Chunk,
        origin: IVec3,
        zs: impl Iterator<Item = u32>,
        f: Face,
        quads: &mut Vec<Quad>,
    ) {
        let visible_mask = &mut self.visible_masks[f];
        for z in zs {
            for y in 1..LEN_U32 - 1 {
                let i_2d = [y, z].i_2d();
                let mut visible = visible_mask[i_2d];

                let upward_visible = visible_mask[i_2d + STRIDE_Y_2D];

                while visible != 0 {
                    let x = visible.trailing_zeros();

                    let upward_i = x as usize;

                    let i_3d = (x, i_2d).i_3d();
                    let voxel_opt = chunk.voxels[i_3d];
                    let voxel = voxel_opt.unwrap();

                    // upward merging
                    if (upward_visible >> x) & 1 != 0
                        && voxel_opt == chunk.voxels[i_3d + STRIDE_Y_3D]
                    {
                        self.upward_merged[upward_i] += 1;
                        visible &= visible - 1;
                        continue;
                    }

                    // rightward merging
                    let mut right_merged = 1;
                    let mut upward_next_i = upward_i;
                    let mut next_i_3d = i_3d;
                    for x in x + 1..LEN_U32 - 1 {
                        upward_next_i += UPWARD_STRIDE_X;
                        next_i_3d += STRIDE_X_3D;

                        if (visible >> x) & 1 == 0
                            || self.upward_merged[upward_i] != self.upward_merged[upward_next_i]
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
                    quads.push({
                        let upward_merged = self.upward_merged[upward_i] as u32;

                        let w = right_merged;
                        let h = upward_merged + 1;

                        let y = y - upward_merged;

                        let pos = uvec3(x, y, z).as_ivec3() + origin;

                        let t = match voxel {
                            Voxel::Liquid => 0,
                            Voxel::Solid => 1,
                        };

                        Quad::new(pos, w, h, f, t)
                    });

                    self.upward_merged[upward_i] = 0;
                }
            }
        }
    }

    pub fn mesh(&mut self, chunk: &Chunk, origin: IVec3) -> ChunkMesh {
        self.build_all_visible_masks(chunk);
        ChunkMesh(self.merged_quads(chunk, origin))
    }
}

impl Mesher {
    pub fn remesh(
        &mut self,
        chunk: &Chunk,
        origin: IVec3,
        mesh: &mut ChunkMesh,
        changes: ChunkMeshChanges,
    ) {
        let remesh = changes.0.map(|x| (x | x << 1 | x >> 1) & !PAD_MASK);

        self.build_visible_masks(chunk, remesh);

        for f in Face::ALL {
            let quads = &mut mesh[f];
            match f {
                PosX | NegX => {
                    self.inner
                        .merge_x(chunk, origin, remesh.x, f, &mut self.quads);

                    let mut src_start = 0;
                    for x in BitIter::from(remesh.x).map(|x| origin.x + x as i32) {
                        let dst_range = key_range(&quads, |q| q.pos.x, x);

                        let src_end = self.quads.partition_point(|q| q.pos.x <= x);
                        let replace_with = self.quads[src_start..src_end].iter().copied();
                        src_start = src_end;

                        quads.splice(dst_range, replace_with);
                    }

                    self.quads.clear();
                }
                PosY | NegY => {
                    self.inner.merge_y(
                        chunk,
                        origin,
                        BitIter::from(remesh.y).map(u32),
                        f,
                        &mut self.quads,
                    );

                    let mut src_start = 0;
                    for y in BitIter::from(remesh.y).map(|y| origin.y + y as i32) {
                        let dst_range = key_range(&quads, |q| q.pos.y, y);

                        let src_end = self.quads.partition_point(|q| q.pos.y <= y);
                        let replace_with = self.quads[src_start..src_end].iter().copied();
                        src_start = src_end;

                        quads.splice(dst_range, replace_with);
                    }

                    self.quads.clear();
                }
                PosZ | NegZ => {
                    self.inner.merge_z(
                        chunk,
                        origin,
                        BitIter::from(remesh.z).map(u32),
                        f,
                        &mut self.quads,
                    );

                    let mut src_start = 0;
                    for z in BitIter::from(remesh.z).map(|z| origin.z + z as i32) {
                        let dst_range = key_range(&quads, |q| q.pos.z, z);

                        let src_end = self.quads.partition_point(|q| q.pos.z <= z);
                        let replace_with = self.quads[src_start..src_end].iter().copied();
                        src_start = src_end;

                        quads.splice(dst_range, replace_with);
                    }

                    self.quads.clear();
                }
            }
        }
    }
}

#[inline]
fn key_range(slice: &[Quad], key: impl Fn(&Quad) -> i32, k: i32) -> Range<usize> {
    let start = slice.partition_point(|q| key(q) < k);
    let len = slice[start..].partition_point(|q| key(q) == k);
    let end = start + len;
    start..end
}

#[inline]
fn u32(usize: usize) -> u32 {
    usize as u32
}
