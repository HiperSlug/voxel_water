mod action;

use std::array;
use std::hash::BuildHasher;

use action::{ACTIONS, Action, DOWN_ACTION};
use bevy::platform::hash::FixedState;
use bit_iter::BitIter;
use dashmap::DashMap;
use rand::rng;
use rand::seq::SliceRandom;

use super::index::{Index2d, Index3d};
use super::{Chunk, LEN_U32, PAD_MASK};

impl Chunk {
    pub fn collect_moves(&self, dst_to_src: &DashMap<usize, usize>, tick: u64) {
        let state = FixedState::with_seed(tick);
        let inv_state = FixedState::with_seed(!tick);

        for z in 1..LEN_U32 - 1 {
            'row: for y in 1..LEN_U32 - 1 {
                let i_2d = [y, z].i_2d();

                let mut liquid = self.masks[i_2d].liquid & !PAD_MASK;

                if liquid == 0 {
                    continue 'row;
                }

                {
                    let moved = self.move_group(liquid, i_2d, &state, &DOWN_ACTION, dst_to_src);

                    liquid &= !moved;

                    if liquid == 0 {
                        continue 'row;
                    }
                }

                let x_mask = state.hash_one(i_2d);
                let pos_mask = inv_state.hash_one(i_2d);

                let group_masks = [
                    x_mask & pos_mask,
                    x_mask & !pos_mask,
                    !x_mask & pos_mask,
                    !x_mask & !pos_mask,
                ];

                for action_group in ACTIONS {
                    let mut is: [_; 4] = array::from_fn(|i| i);
                    is.shuffle(&mut rng());
                    for i in is {
                        for j in 0..4 {
                            let group = liquid & group_masks[(i + j) % 4];
                            if group == 0 {
                                continue;
                            }

                            let moved =
                                self.move_group(group, i_2d, &state, &action_group[j], dst_to_src);

                            liquid &= !moved;

                            if liquid == 0 {
                                continue 'row;
                            }
                        }
                    }
                }
            }
        }
    }

    fn move_group(
        &self,
        group: u64,
        src_i_2d: usize,
        state: &FixedState,
        action: &Action,
        dst_to_src: &DashMap<usize, usize>,
    ) -> u64 {
        let (delta, prereqs) = action;

        let (d_x, d_i_2d) = delta.x_and_i_2d();
        let d_i_3d = delta.i_3d();

        let mut prereq_mask = !0;
        for prereq in *prereqs {
            let (d_x, d_i_2d) = prereq.delta.x_and_i_2d();
            let i_2d = src_i_2d.wrapping_add_signed(d_i_2d);
            let mask = self.masks[i_2d].occupied.inv_shift(d_x);

            prereq_mask &= if prereq.not { !mask } else { mask };
        }

        let dst_i_2d = src_i_2d.wrapping_add_signed(d_i_2d);

        let try_move = group & prereq_mask & !self.masks[dst_i_2d].occupied.inv_shift(d_x);

        for x in BitIter::from(try_move) {
            let src_i_3d = (x, src_i_2d).i_3d();
            let dst_i_3d = src_i_3d.wrapping_add_signed(d_i_3d);

            dst_to_src
                .entry(dst_i_3d)
                .and_modify(|other_src_i_3d| {
                    let priority = state.hash_one(src_i_3d);
                    let other_priority = state.hash_one(*other_src_i_3d);

                    if priority > other_priority {
                        *other_src_i_3d = src_i_3d;
                    }
                })
                .or_insert(src_i_3d);
        }

        try_move
    }
}

trait Shift: Copy {
    // /// shl
    // fn shift(self, rhs: isize) -> u64;

    /// shr
    fn inv_shift(self, rhs: isize) -> u64;
}

impl Shift for u64 {
    // fn shift(self, rhs: isize) -> u64 {
    //     let mut out = self.wrapping_shr(-rhs as u32);
    //     if rhs > 0 {
    //         out = self.wrapping_shl(rhs as u32)
    //     }
    //     out
    // }

    fn inv_shift(self, rhs: isize) -> u64 {
        let mut out = self.wrapping_shl(-rhs as u32);
        if rhs > 0 {
            out = self.wrapping_shr(rhs as u32)
        }
        out
    }
}
