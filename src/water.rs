use bevy::prelude::*;
use itertools::Either;
use rand::{Rng, seq::SliceRandom};

use crate::chunk::{Chunk, LEN_U32, PAD_MASK, STRIDE_0, STRIDE_1, linearize_2d};

impl Chunk {
    pub fn liquid_tick(&mut self) {
        const STRIDE_Y: isize = STRIDE_0 as isize;
        const STRIDE_Z: isize = STRIDE_1 as isize;

        let [front, back] = self.masks.buffers_mut();

        // TODO: determine if this makes a difference
        let range = if self.prng.random() {
            Either::Left(1..LEN_U32 - 1)
        } else {
            Either::Right((1..LEN_U32 - 1).rev())
        };

        for z in range.clone() {
            'a: for y in range.clone() {
                let i = linearize_2d([y, z]);

                let pad_some = front.some_mask[i];
                let mut some = pad_some & !PAD_MASK;

                if some == 0 {
                    continue;
                }

                // down
                let down_i = (i as isize - STRIDE_Y) as usize;
                let fall = some & !front.some_mask[down_i] & !back.some_mask[down_i];
                some &= !fall;
                back.some_mask[down_i] |= fall;

                if some == 0 {
                    continue;
                }

                use Dir::*;
                #[derive(Clone, Copy)]
                enum Dir {
                    PosX,
                    NegX,
                    PosZ,
                    NegZ,
                }

                // TODO: batch into 4 groups for cellular randomness and drop shuffling. priority wouldn't need to change for each phase.

                let mut dirs = [PosX, NegX, PosZ, NegZ];
                dirs.shuffle(&mut self.prng);

                // adjacent
                for dir in dirs {
                    match dir {
                        PosX => {
                            let fall = (some >> 1) & !front.some_mask[down_i] &!back.some_mask[down_i];
                            some &= !(fall << 1);
                            back.some_mask[down_i] |= fall;
                        },
                        NegX => {
                            let fall = (some << 1) & !front.some_mask[down_i] &!back.some_mask[down_i];
                            some &= !(fall >> 1);
                            back.some_mask[down_i] |= fall;
                        },
                        PosZ => {
                            let i = (down_i as isize + STRIDE_Z) as usize;
                            let fall = some & !front.some_mask[i] & !back.some_mask[i];
                            some &= !fall;
                            back.some_mask[i] |= fall;
                        },
                        NegZ => {
                            let i = (down_i as isize - STRIDE_Z) as usize;
                            let fall = some & !front.some_mask[i] & !back.some_mask[i];
                            some &= !fall;
                            back.some_mask[i] |= fall;
                        },
                    }
                    if some == 0 {
                        continue 'a;
                    }
                }

                dirs.shuffle(&mut self.prng);

                // diagonal
                for dir in dirs {
                    match dir {
                        PosX => {
                            let i = (down_i as isize - STRIDE_Z) as usize;
                            let fall = (some >> 1) & !front.some_mask[i] & !back.some_mask[i];
                            some &= !(fall << 1);
                            back.some_mask[i] |= fall;
                        },
                        NegX => {
                            let i = (down_i as isize + STRIDE_Z) as usize;
                            let fall = (some << 1) & !front.some_mask[i] & !back.some_mask[i];
                            some &= !(fall >> 1);
                            back.some_mask[i] |= fall;
                        },
                        PosZ => {
                            let i = (down_i as isize + STRIDE_Z) as usize;
                            let fall = (some >> 1) & !front.some_mask[i] & !back.some_mask[i];
                            some &= !(fall << 1);
                            back.some_mask[i] |= fall;
                        },
                        NegZ => {
                            let i = (down_i as isize - STRIDE_Z) as usize;
                            let fall = (some << 1) & !front.some_mask[i] & !back.some_mask[i];
                            some &= !(fall >> 1);
                            back.some_mask[i] |= fall;
                        },
                    }
                    if some == 0 {
                        continue 'a;
                    }
                }

                dirs.shuffle(&mut self.prng);

                // lateral
                for dir in dirs {
                    match dir {
                        PosX => {
                            let slide = some & !(front.some_mask[i] << 1) & (front.some_mask[i] >> 1) & (back.some_mask[i] << 1);
                            some &= !slide;
                            back.some_mask[i] |= slide >> 1;
                        },
                        NegX => {
                            let slide = some & !(front.some_mask[i] >> 1) & (front.some_mask[i] << 1) & (back.some_mask[i] >> 1);
                            some &= !slide;
                            back.some_mask[i] |= slide << 1;
                        },
                        PosZ => {
                            let inv_i = (i as isize - STRIDE_Z) as usize;
                            let i = (i as isize + STRIDE_Z) as usize;
                            let slide = some & !front.some_mask[i] & front.some_mask[inv_i] & !back.some_mask[i];
                            some &= !slide;
                            back.some_mask[i] |= slide;
                        },
                        NegZ => {
                            let inv_i = (i as isize + STRIDE_Z) as usize;
                            let i = (i as isize - STRIDE_Z) as usize;
                            let slide = some & !front.some_mask[i] & front.some_mask[inv_i] & !back.some_mask[i];
                            some &= !slide;
                            back.some_mask[i] |= slide;
                        },
                    }
                    if some == 0 {
                        continue 'a;
                    }
                }
            }
        }

        // preserving padding
        for z in [0, LEN_U32 - 1] {
            for y in 0..LEN_U32 {
                let i = linearize_2d([y, z]);

                back.some_mask[i] |= front.some_mask[i]
            }
        }

        for z in 0..LEN_U32 {
            for y in [0, LEN_U32 - 1] {
                let i = linearize_2d([y, z]);

                back.some_mask[i] |= front.some_mask[i]
            }
        }

        *front = default();
        self.masks.swap();
    }
}
