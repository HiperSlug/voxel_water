use bevy::prelude::*;
use itertools::Either;
use rand::random;

use crate::chunk::{Chunk, LEN_U32, PAD_MASK, STRIDE_0, STRIDE_1, linearize_2d};

impl Chunk {
    // TODO: batch into 4 groups for cellular randomness and drop shuffling. priority wouldn't need to change for each phase.
    // BIG_TODO: update `voxels` as well as masks
    // TODO: cheaper `rng()`
    pub fn liquid_tick(&mut self) {
        const STRIDE_Y: isize = STRIDE_0 as isize;
        const STRIDE_Z: isize = STRIDE_1 as isize;

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

                let pad_some = read.some_mask[i];
                let mut some = pad_some & !PAD_MASK;

                if some == 0 {
                    continue;
                }

                // down
                let ny_i = (i as isize - STRIDE_Y) as usize;
                let fall = some & !read.some_mask[ny_i] & !write.some_mask[ny_i];
                some &= !fall;
                write.some_mask[ny_i] |= fall;

                if some == 0 {
                    continue 'row;
                }

                let ny_pz_i = (ny_i as isize + STRIDE_Z) as usize;
                let ny_nz_i = (ny_i as isize - STRIDE_Z) as usize;

                // random priorities
                let x_mask = random::<u64>();
                let pos_mask = random::<u64>();

                let masks = [
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
                        let mask = masks[(i + j) % 4];

                        match dir {
                            PosX => {
                                let fall = ((some & mask) >> 1)
                                    & !read.some_mask[ny_i]
                                    & !write.some_mask[ny_i];
                                some &= !(fall << 1);
                                write.some_mask[ny_i] |= fall;
                            }
                            NegX => {
                                let fall = ((some & mask) << 1)
                                    & !read.some_mask[ny_i]
                                    & !write.some_mask[ny_i];
                                some &= !(fall >> 1);
                                write.some_mask[ny_i] |= fall;
                            }
                            PosZ => {
                                let fall = some
                                    & mask
                                    & !read.some_mask[ny_pz_i]
                                    & !write.some_mask[ny_pz_i];
                                some &= !fall;
                                write.some_mask[ny_pz_i] |= fall;
                            }
                            NegZ => {
                                let fall = some
                                    & mask
                                    & !read.some_mask[ny_nz_i]
                                    & !write.some_mask[ny_nz_i];
                                some &= !fall;
                                write.some_mask[ny_nz_i] |= fall;
                            }
                        }
                        if some == 0 {
                            continue 'row;
                        }
                    }
                }

                // diagonal
                for i in 0..4 {
                    for j in 0..4 {
                        let dir = DIRS[j];
                        let mask = masks[(i + j) % 4];

                        match dir {
                            PosX => {
                                let fall = ((some & mask) >> 1)
                                    & !read.some_mask[ny_nz_i]
                                    & !write.some_mask[ny_nz_i];
                                some &= !(fall << 1);
                                write.some_mask[ny_nz_i] |= fall;
                            }
                            NegX => {
                                let fall = ((some & mask) << 1)
                                    & !read.some_mask[ny_pz_i]
                                    & !write.some_mask[ny_pz_i];
                                some &= !(fall >> 1);
                                write.some_mask[ny_pz_i] |= fall;
                            }
                            PosZ => {
                                let fall = ((some & mask) >> 1)
                                    & !read.some_mask[ny_pz_i]
                                    & !write.some_mask[ny_pz_i];
                                some &= !(fall << 1);
                                write.some_mask[ny_pz_i] |= fall;
                            }
                            NegZ => {
                                let fall = ((some & mask) << 1)
                                    & !read.some_mask[ny_nz_i]
                                    & !write.some_mask[ny_nz_i];
                                some &= !(fall >> 1);
                                write.some_mask[ny_nz_i] |= fall;
                            }
                        }
                    }
                }

                let pz_i = (i as isize + STRIDE_Y) as usize;
                let nz_i = (i as isize - STRIDE_Y) as usize;

                // lateral
                for i in 0..4 {
                    for j in 0..4 {
                        let dir = DIRS[j];
                        let mask = masks[(i + j) % 4];

                        match dir {
                            PosX => {
                                let slide = some
                                    & mask
                                    & !(read.some_mask[i] << 1)
                                    & (read.some_mask[i] >> 1)
                                    & (write.some_mask[i] << 1);
                                some &= !slide;
                                write.some_mask[i] |= slide >> 1;
                            }
                            NegX => {
                                let slide = some
                                    & mask
                                    & !(read.some_mask[i] >> 1)
                                    & (read.some_mask[i] << 1)
                                    & (write.some_mask[i] >> 1);
                                some &= !slide;
                                write.some_mask[i] |= slide << 1;
                            }
                            PosZ => {
                                let slide = some
                                    & mask
                                    & !read.some_mask[pz_i]
                                    & read.some_mask[nz_i]
                                    & !write.some_mask[pz_i];
                                some &= !slide;
                                write.some_mask[pz_i] |= slide;
                            }
                            NegZ => {
                                let slide = some
                                    & mask
                                    & !read.some_mask[nz_i]
                                    & read.some_mask[pz_i]
                                    & !write.some_mask[nz_i];
                                some &= !slide;
                                write.some_mask[nz_i] |= slide;
                            }
                        }
                    }
                }
            }
        }

        // preserving padding in a single chunk simulation
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
