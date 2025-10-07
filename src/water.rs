use bevy::prelude::*;

use crate::chunk::{linearize_2d, Chunk, LEN, PAD_MASK, STRIDE_0, STRIDE_1};

// double buffering with first-come-first-serve

#[derive(Default, Resource)]
pub struct DoubleBuffered {
    pub chunks: [Chunk; 2],
    state: bool,
}

impl DoubleBuffered {
    pub fn current(&self) -> &Chunk {
        let read_i = self.state as usize;
        &self.chunks[read_i]
    }

    pub fn current_mut(&mut self) -> &mut Chunk {
        let read_i = self.state as usize;
        &mut self.chunks[read_i]
    }

    pub fn tick(&mut self) {
        const STRIDE_Y: usize = STRIDE_0;
        const STRIDE_Z: usize = STRIDE_1;

        let read_i = self.state as usize;
        let write_i: usize = (!self.state) as usize;

        for z in 1..LEN as u32 - 1 {
            for y in 1..LEN as u32 - 1 {
                let i = linearize_2d([y, z]);

                let pad_some = self.chunks[read_i].some_mask[i];
                let mut some = pad_some & !PAD_MASK;

                for offset in [
                    -(STRIDE_Y as isize),
                    -(STRIDE_Y as isize) - STRIDE_Z as isize,
                    -(STRIDE_Y as isize) + STRIDE_Z as isize,
                ] {
                    let adj_i = (i as isize + offset) as usize;
                    let r_adj_some = self.chunks[read_i].some_mask[adj_i];
                    let w_adj_some = &mut self.chunks[write_i].some_mask[adj_i];

                    let fall = some & !r_adj_some & !*w_adj_some;
                    some &= !fall;
                    *w_adj_some |= fall;

                    if some == 0 {
                        break;
                    }

                    let fall_left = (some << 1) & !r_adj_some & !*w_adj_some;
                    some &= !(fall_left >> 1);
                    *w_adj_some |= fall_left;

                    if some == 0 {
                        break;
                    }

                    let fall_right = (some >> 1) & !r_adj_some & !*w_adj_some;
                    some &= !(fall_right << 1);
                    *w_adj_some |= fall_right;

                    if some == 0 {
                        break;
                    }
                }
                self.chunks[write_i].some_mask[i] |= some | (PAD_MASK & pad_some);
            }
        }

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
        self.chunks[read_i] = default();

        self.state = !self.state;
    }
}
