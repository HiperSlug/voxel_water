use bevy::prelude::*;
use itertools::Either;
use rand::{random, seq::SliceRandom};
use std::ops::Range;

use crate::chunk::{
    Chunk, LEN_U32, Masks, PAD_MASK, STRIDE_0, STRIDE_1, Voxel, Voxels, linearize_2d,
};

impl Chunk {
    // fn zero_and_swap(&mut self) {
    //     if self.state {
    //         self.masks[0] = default();
    //     } else {
    //         self.masks[1] = default();
    //     }
    //     self.state = !self.state
    // }

    // fn read_write_voxels(&mut self) -> (&Masks, &mut Masks, &mut Voxels) {
    //     let (left, right) = self.masks.split_at_mut(1);
    //     if self.state {
    //         (&left[0], &mut right[0], &mut self.voxels)
    //     } else {
    //         (&right[0], &mut left[0], &mut self.voxels)
    //     }
    // }

    // pub fn set(&mut self, p: impl Into<[u32; 3]> + Copy, v: Option<Voxel>) {
    //     if self.state {
    //         self.masks[0].set(p, v);
    //     } else {
    //         self.masks[1].set(p, v);
    //     }
    //     self.voxels.set(p, v);
    // }

    pub fn liquid_tick(&mut self) {
        const STRIDE_Y: isize = STRIDE_0 as isize;
        const STRIDE_Z: isize = STRIDE_1 as isize;

        let (read, write, voxels) = self.read_write_voxels();

        const RANGE: Range<u32> = 1..LEN_U32 - 1;
        let range = if random() {
            Either::Left(RANGE)
        } else {
            Either::Right(RANGE.rev())
        };

        for z in range.clone() {
            'outer: for y in range.clone() {
                let i = linearize_2d([y, z]);

                let pad_some = read.some_mask[i];
                let mut some = pad_some & !PAD_MASK;

                if some == 0 {
                    continue;
                }

                let mut offsets = [-STRIDE_Y - STRIDE_Z, -STRIDE_Y + STRIDE_Z];
                offsets.shuffle(&mut rand::rng());

                for offset in [-STRIDE_Y].into_iter().chain(offsets) {
                    let adj_i = (i as isize + offset) as usize;
                    let r_adj_some = read.some_mask[adj_i];
                    let w_adj_some = &mut write.some_mask[adj_i];

                    let fall = some & !r_adj_some & !*w_adj_some;
                    some &= !fall;
                    *w_adj_some |= fall;

                    if random() {
                        let fall_left = (some << 1) & !r_adj_some & !*w_adj_some;
                        some &= !(fall_left >> 1);
                        *w_adj_some |= fall_left;

                        let fall_right = (some >> 1) & !r_adj_some & !*w_adj_some;
                        some &= !(fall_right << 1);
                        *w_adj_some |= fall_right;
                    } else {
                        let fall_right = (some >> 1) & !r_adj_some & !*w_adj_some;
                        some &= !(fall_right << 1);
                        *w_adj_some |= fall_right;

                        let fall_left = (some << 1) & !r_adj_some & !*w_adj_some;
                        some &= !(fall_left >> 1);
                        *w_adj_some |= fall_left;
                    }

                    // we dont need to push nothing to the zeroed write buffer
                    if some == 0 {
                        continue 'outer;
                    }
                }

                let z_mask = random::<u64>();
                let x_mask = !z_mask;

                let mut offsets = [STRIDE_Z, -STRIDE_Z];
                offsets.shuffle(&mut rand::rng());

                // +- Z
                for offset in offsets {
                    let inv_adj_i = (i as isize - offset) as usize;
                    let r_inv_adj_some = read.some_mask[inv_adj_i];

                    let adj_i = (i as isize + offset) as usize;
                    let r_adj_some = read.some_mask[adj_i];
                    let w_adj_some = &mut write.some_mask[adj_i];

                    let shift = some & z_mask & !r_adj_some & !*w_adj_some & r_inv_adj_some;

                    some &= !shift;
                    *w_adj_some |= shift;

                    if some == 0 {
                        continue 'outer;
                    }
                }

                let r_adj_some = pad_some;
                let w_adj_some = &mut write.some_mask[i];

                if random() {
                    // + X
                    let adj_shift = {
                        let r_inv_adj_some = r_adj_some >> 1;
                        let r_adj_some = r_adj_some << 1;
                        let w_adj_some = *w_adj_some << 1;

                        some & x_mask & !r_adj_some & !w_adj_some & r_inv_adj_some
                    };
                    some &= !adj_shift;
                    *w_adj_some |= adj_shift >> 1;

                    // - X
                    let adj_shift = {
                        let r_inv_adj_some = r_adj_some << 1;
                        let r_adj_some = r_adj_some >> 1;
                        let w_adj_some = *w_adj_some >> 1;

                        some & x_mask & !r_adj_some & !w_adj_some & r_inv_adj_some
                    };
                    some &= !adj_shift;
                    *w_adj_some |= adj_shift << 1;
                } else {
                    // - X
                    let adj_shift = {
                        let r_inv_adj_some = r_adj_some << 1;
                        let r_adj_some = r_adj_some >> 1;
                        let w_adj_some = *w_adj_some >> 1;

                        some & x_mask & !r_adj_some & !w_adj_some & r_inv_adj_some
                    };
                    some &= !adj_shift;
                    *w_adj_some |= adj_shift << 1;

                    // + X
                    let adj_shift = {
                        let r_inv_adj_some = r_adj_some >> 1;
                        let r_adj_some = r_adj_some << 1;
                        let w_adj_some = *w_adj_some << 1;

                        some & x_mask & !r_adj_some & !w_adj_some & r_inv_adj_some
                    };
                    some &= !adj_shift;
                    *w_adj_some |= adj_shift >> 1;
                }

                // we only push remaining cells
                write.some_mask[i] |= some | (PAD_MASK & pad_some);
            }
        }

        // keep padding b/c single chunk simulation
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

        self.zero_and_swap();
    }
}
