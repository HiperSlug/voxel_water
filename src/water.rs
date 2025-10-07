use bevy::prelude::*;

use crate::chunk::{linearize_2d, Chunk, LEN, STRIDE_0, STRIDE_1};

// double buffering with first-come-first-serve

#[derive(Default, Resource)]
pub struct DoubleBuffered {
    chunks: [Chunk; 2],
    state: bool,
}

impl DoubleBuffered {
    pub fn current(&self) -> &Chunk {
        let read_i = self.state as usize;
        &self.chunks[read_i]
    }
    
    pub fn tick(&mut self) {
        const STRIDE_Y: usize = STRIDE_0;
        const STRIDE_Z: usize = STRIDE_1;

        let read_i = self.state as usize;
        let write_i = (!self.state) as usize;

        for z in 1..LEN as u32 - 1 {
            for y in 1..LEN as u32 - 1 {
                let i = linearize_2d([y, z]);

                let mut some = self.chunks[read_i].some_mask[i];

                let mut down_some = self.chunks[read_i].some_mask[i - STRIDE_Y];

                let mut down_forward_some = self.chunks[read_i].some_mask[i - STRIDE_Y + STRIDE_Z];
                let mut down_backward_some = self.chunks[read_i].some_mask[i - STRIDE_Y - STRIDE_Z];
                
                // TODO: lazy mem fetching
                // TODO: sync mem writes
                // TODO: DRY it up
                // TODO: fall diagonal
                // TODO: move horizontally
                // TODO: handle collisions

                // down
                let fall_down = some & !down_some;
                some &= !fall_down;
                down_some |= fall_down;

                // down_left
                let fall_down_left = (some << 1) & !down_some; 
                some &= !(fall_down_left << 1);
                down_some |= fall_down_left;

                // down_right
                let fall_down_right = (some >> 1) & !down_some; // shl b/c we are moving dst to ourselves instead of ourselves to dst.
                some &= !(fall_down_right >> 1);
                down_some |= fall_down_right;

                // down_forward
                let fall_down_forward = some & !down_forward_some;
                some &= !fall_down_forward;
                down_forward_some |= fall_down_forward;

                // down_backward
                let fall_down_backward = some & !down_backward_some;
                some &= !fall_down_backward;
                down_backward_some |= fall_down_backward;

                self.chunks[write_i].some_mask[i] = some;
                self.chunks[write_i].some_mask[i - STRIDE_Y] |= down_some;
                self.chunks[write_i].some_mask[i - STRIDE_Y + STRIDE_Z] |= down_forward_some;
                self.chunks[write_i].some_mask[i - STRIDE_Y - STRIDE_Z] |= down_backward_some;
            }
        }
        self.state = !self.state;
    }
}