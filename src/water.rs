use std::ops::Range;
use bevy::prelude::*;
use itertools::Either;
use rand::random;

use crate::chunk::{Chunk, LEN, PAD_MASK, STRIDE_0, STRIDE_1, linearize_2d};

#[derive(Default, Resource)]
pub struct DoubleBuffered {
    chunks: [Chunk; 2],
    state: bool,
}

impl DoubleBuffered {
    pub fn front(&self) -> &Chunk {
        let read_i = self.state as usize;
        &self.chunks[read_i]
    }

    pub fn front_mut(&mut self) -> &mut Chunk {
        let read_i = self.state as usize;
        &mut self.chunks[read_i]
    }

    pub fn tick(&mut self) {
        const STRIDE_Y: isize = STRIDE_0 as isize;
        const STRIDE_Z: isize = STRIDE_1 as isize;

        let read_i = self.state as usize;
        let write_i = (!self.state) as usize;

        const RANGE: Range<u32> = 1..LEN as u32 - 1;
        let range = if random() {
            Either::Left(RANGE)
        } else {
            Either::Right(RANGE.rev())
        };

        for z in range.clone() {
            for y in range.clone() {
                let y_state = y % 2 == 0;

                let i = linearize_2d([y, z]);

                let pad_some = self.chunks[read_i].some_mask[i];
                let mut some = pad_some & !PAD_MASK;

                const OFFSETS: [isize; 2] = [
                    -STRIDE_Y - STRIDE_Z,
                    -STRIDE_Y + STRIDE_Z,
                ];
                let offsets = if y_state {
                    Either::Left(OFFSETS.into_iter())
                } else {
                    Either::Right(OFFSETS.into_iter().rev())
                };

                for offset in [-STRIDE_Y].into_iter().chain(offsets) {
                    let adj_i = (i as isize + offset) as usize;
                    let r_adj_some = self.chunks[read_i].some_mask[adj_i];
                    let w_adj_some = &mut self.chunks[write_i].some_mask[adj_i];

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
                }

                let random_mask = random::<u64>();

                let z_mask = random::<u64>();
                let x_mask = !z_mask;

                // +- Z
                for offset in [STRIDE_Z, -STRIDE_Z] {
                    let inv_adj_i = (i as isize - offset) as usize;
                    let r_inv_adj_some = self.chunks[read_i].some_mask[inv_adj_i];

                    let adj_i = (i as isize + offset) as usize;
                    let r_adj_some = self.chunks[read_i].some_mask[adj_i];
                    let w_adj_some = &mut self.chunks[write_i].some_mask[adj_i];

                    let shift = some & z_mask & random_mask & !r_adj_some & !*w_adj_some & r_inv_adj_some;

                    some &= !shift;
                    *w_adj_some |= shift;
                }

                let r_adj_some = pad_some;
                let w_adj_some = &mut self.chunks[write_i].some_mask[i];

                // + X
                let adj_shift = {
                    let r_inv_adj_some = r_adj_some >> 1;
                    let r_adj_some = r_adj_some << 1;
                    let w_adj_some = *w_adj_some << 1;

                    some & x_mask & random_mask & !r_adj_some & !w_adj_some & r_inv_adj_some
                };
                some &= !adj_shift;
                *w_adj_some |= adj_shift >> 1;

                // - X
                let adj_shift = {
                    let r_inv_adj_some = r_adj_some << 1;
                    let r_adj_some = r_adj_some >> 1;
                    let w_adj_some = *w_adj_some >> 1;

                    some & x_mask & random_mask & !r_adj_some & !w_adj_some & r_inv_adj_some
                };
                some &= !adj_shift;
                *w_adj_some |= adj_shift << 1;

                self.chunks[write_i].some_mask[i] |= some | (PAD_MASK & pad_some);
            }
        }

        // keep padding b/c single chunk simulation
        for z in [0, LEN as u32 - 1] {
            for y in 0..LEN as u32 {
                let i = linearize_2d([y, z]);

                self.chunks[write_i].some_mask[i] |= self.chunks[read_i].some_mask[i]
            }
        }

        for z in 0..LEN as u32 {
            for y in [0, LEN as u32 - 1] {
                let i = linearize_2d([y, z]);

                self.chunks[write_i].some_mask[i] |= self.chunks[read_i].some_mask[i]
            }
        }

        // zero next write chunk
        self.chunks[read_i] = default();

        self.state = !self.state;
    }
}
