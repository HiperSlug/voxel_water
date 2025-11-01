mod action;

use bevy::platform::hash::FixedState;
use bit_iter::BitIter;
use std::hash::BuildHasher;

use action::{ACTIONS, Action, DOWN_ACTION};

use super::index::{Index2d, Index3d};
use super::{Chunk, LEN_U32, PAD_MASK};

impl Chunk {
    pub fn liquid_tick(&mut self, tick: u64) {
        let state = FixedState::with_seed(tick);
        let inv_state = FixedState::with_seed(!tick);

        for z in 1..LEN_U32 - 1 {
            'row: for y in 1..LEN_U32 - 1 {
                let i_2d = [y, z].i_2d();

                let mut liquid = self.masks.dblt_masks.front.liquid_mask[i_2d] & !PAD_MASK;

                if liquid == 0 {
                    continue 'row;
                }

                {
                    let moved = self.try_move_row(liquid, i_2d, &state, &DOWN_ACTION);

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
                    for i in 0..4 {
                        for j in 0..4 {
                            let group = liquid & group_masks[(i + j) % 4];
                            if group == 0 {
                                continue;
                            }

                            let moved = self.try_move_row(group, i_2d, &state, &action_group[j]);

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

    fn try_move_row(
        &mut self,
        group: u64,
        src_i_2d: usize,
        state: &FixedState,
        action: &Action,
    ) -> u64 {
        let (delta, prereqs) = action;

        let (d_x, d_i_2d) = delta.x_and_i_2d();
        let d_i_3d = delta.i_3d();

        let mut prereq_mask = !0;
        for prereq in *prereqs {
            let (x, i_2d) = prereq.delta.x_and_i_2d();
            let i_2d = src_i_2d.wrapping_add_signed(i_2d);
            let mask = self.masks.dblt_masks.front.some_mask[i_2d].inv_shift(x);

            prereq_mask &= if prereq.not { !mask } else { mask };
        }

        let dst_i_2d = src_i_2d.wrapping_add_signed(d_i_2d);

        let try_move =
            group & prereq_mask & !self.masks.dblt_masks.front.some_mask[dst_i_2d].inv_shift(d_x);

        let success = try_move & !self.masks.dblt_masks.back.some_mask[dst_i_2d].inv_shift(d_x);
        let failure = try_move & !success;

        let mut moved = success;

        if success != 0 {
            let add_mask = success.shift(d_x);

            self.masks.dblt_masks.back.some_mask[dst_i_2d] |= add_mask;
            self.masks.dblt_masks.back.liquid_mask[dst_i_2d] |= add_mask;

            self.masks.dblt_masks.back.some_mask[src_i_2d] &= !success;
            self.masks.dblt_masks.back.liquid_mask[src_i_2d] &= !success;
        }

        for x in BitIter::from(success) {
            let src_i_3d = (x, src_i_2d).i_3d();
            let dst_i_3d = src_i_3d.wrapping_add_signed(d_i_3d);

            self.voxels[dst_i_3d] = self.voxels[src_i_3d];
            self.voxels[src_i_3d] = None;

            self.dst_to_src.insert(dst_i_3d, src_i_3d);
        }

        for x in BitIter::from(failure) {
            let src_i_3d = (x, src_i_2d).i_3d();
            let dst_i_3d = src_i_3d.wrapping_add_signed(d_i_3d);

            let other_src_i_3d = self.dst_to_src.get_mut(&dst_i_3d).unwrap();

            let priority = state.hash_one(src_i_3d);
            let other_priority = state.hash_one(*other_src_i_3d);

            if priority >= other_priority {
                let src_bit = u64::bit(x);
                moved |= src_bit;

                let (other_x, other_i_2d) = other_src_i_3d.x_and_i_2d();
                let other_bit = u64::bit(other_x as usize);

                self.masks.dblt_masks.back.liquid_mask[other_i_2d] |= other_bit;
                self.masks.dblt_masks.back.some_mask[other_i_2d] |= other_bit;
                self.masks.dblt_masks.back.liquid_mask[src_i_2d] &= !src_bit;
                self.masks.dblt_masks.back.some_mask[src_i_2d] &= !src_bit;

                self.voxels[*other_src_i_3d] = self.voxels[src_i_3d];
                self.voxels[src_i_3d] = None;

                *other_src_i_3d = src_i_3d;
            }
        }

        moved
    }
}

trait Shift: Copy {
    const ONE: Self;
    
    /// shl
    fn shift(self, rhs: isize) -> u64;

    /// shr
    fn inv_shift(self, rhs: isize) -> u64;

    #[inline]
    fn bit(n: usize) -> u64 {
        Self::ONE.shift(n as isize)
    }
}

impl Shift for u64 {
    const ONE: Self = 1;

    fn shift(self, rhs: isize) -> u64 {
        let mut out = self.wrapping_shr(-rhs as u32);
        if rhs > 0 {
            out = self.wrapping_shl(rhs as u32)
        }
        out
    }

    fn inv_shift(self, rhs: isize) -> u64 {
        let mut out = self.wrapping_shl(-rhs as u32);
        if rhs > 0 {
            out = self.wrapping_shr(rhs as u32)
        }
        out
    }
}
