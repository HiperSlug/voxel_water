use bevy::prelude::*;
use itertools::Either;
use rand::random;

use crate::chunk::{
    Chunk, LEN_U32, PAD_MASK, STRIDE_0, STRIDE_1, STRIDE_2, VOL, Voxel, linearize_2d, linearize_3d,
};

impl Chunk {
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
                for i in 0..4 {
                    for j in 0..4 {
                        let dir = DIRS[j];
                        let group_mask = group_masks[(i + j) % 4];

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
                for i in 0..4 {
                    for j in 0..4 {
                        let dir = DIRS[j];
                        let group_mask = group_masks[(i + j) % 4];

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
                for i in 0..4 {
                    for j in 0..4 {
                        let dir = DIRS[j];
                        let group_mask = group_masks[(i + j) % 4];

                        let group = liquid & group_mask;
                        if group == 0 {
                            continue;
                        }

                        match dir {
                            PosX => {
                                let slide = group
                                    & !(read.some_mask[i] << 1)
                                    & (read.some_mask[i] >> 1)
                                    & (write.some_mask[i] << 1);
                                liquid &= !slide;
                                write.some_mask[i] |= slide >> 1;

                                push_2d_to_3d::<STRIDE_X_3D>(&mut self.voxels, slide, yz_base)
                            }
                            NegX => {
                                let slide = group
                                    & !(read.some_mask[i] >> 1)
                                    & (read.some_mask[i] << 1)
                                    & (write.some_mask[i] >> 1);
                                liquid &= !slide;
                                write.some_mask[i] |= slide << 1;

                                push_2d_to_3d::<{ -STRIDE_X_3D }>(&mut self.voxels, slide, yz_base)
                            }
                            PosZ => {
                                let slide = group
                                    & !read.some_mask[pz_i]
                                    & read.some_mask[nz_i]
                                    & !write.some_mask[pz_i];
                                liquid &= !slide;
                                write.some_mask[pz_i] |= slide;

                                push_2d_to_3d::<STRIDE_Z_3D>(&mut self.voxels, slide, yz_base)
                            }
                            NegZ => {
                                let slide = group
                                    & !read.some_mask[nz_i]
                                    & read.some_mask[pz_i]
                                    & !write.some_mask[nz_i];
                                liquid &= !slide;
                                write.some_mask[nz_i] |= slide;

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

        for z in 0..LEN_U32 {
            for y in [0, LEN_U32 - 1] {
                let i = linearize_2d([y, z]);

                write.some_mask[i] |= read.some_mask[i]
            }
        }

        // zero the old read buffer
        *read = default();
    }
}

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
