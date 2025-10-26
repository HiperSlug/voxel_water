use bevy::platform::hash::FixedState;
use bit_iter::BitIter;
use std::hash::BuildHasher;

use super::*;

const I_STRIDE_Y_2D: isize = STRIDE_Y_2D as isize;
const I_STRIDE_Z_2D: isize = STRIDE_Z_2D as isize;

const I_STRIDE_X_3D: isize = STRIDE_X_3D as isize;
const I_STRIDE_Y_3D: isize = STRIDE_Y_3D as isize;
const I_STRIDE_Z_3D: isize = STRIDE_Z_3D as isize;

const MOVES: &[&[(Delta, &[PreReq])]] = &[
    &[
        (Delta::new([1, -1, 0]), &[PreReq::none([1, 0, 0])]),
        (Delta::new([-1, -1, 0]), &[PreReq::none([-1, 0, 0])]),
        (Delta::new([0, -1, 1]), &[PreReq::none([0, 0, 1])]),
        (Delta::new([0, -1, -1]), &[PreReq::none([0, 0, -1])]),
    ],
    &[
        (
            Delta::new([1, -1, 1]),
            &[
                PreReq::none([1, 0, 0]),
                PreReq::none([0, 0, 1]),
                PreReq::none([1, 0, 1]),
            ],
        ),
        (
            Delta::new([-1, -1, 1]),
            &[
                PreReq::none([-1, 0, 0]),
                PreReq::none([0, 0, 1]),
                PreReq::none([-1, 0, 1]),
            ],
        ),
        (
            Delta::new([1, -1, -1]),
            &[
                PreReq::none([1, 0, 0]),
                PreReq::none([0, 0, -1]),
                PreReq::none([1, 0, -1]),
            ],
        ),
        (
            Delta::new([-1, -1, -1]),
            &[
                PreReq::none([-1, 0, 0]),
                PreReq::none([0, 0, -1]),
                PreReq::none([-1, 0, -1]),
            ],
        ),
    ],
    &[
        (Delta::new([1, 0, 0]), &[PreReq::some([-1, 0, 0])]),
        (Delta::new([-1, 0, 0]), &[PreReq::some([-1, 0, 0])]),
        (Delta::new([0, 0, 1]), &[PreReq::some([0, 0, -1])]),
        (Delta::new([0, 0, -1]), &[PreReq::some([0, 0, 1])]),
    ],
];

struct Delta {
    x: isize,
    i_2d: isize,
    i_3d: isize,
}

impl Delta {
    const fn new([x, y, z]: [isize; 3]) -> Self {
        Self {
            i_3d: x * I_STRIDE_X_3D + y * I_STRIDE_Y_3D + z * I_STRIDE_Z_3D,
            i_2d: y * I_STRIDE_Y_2D + z * I_STRIDE_Z_2D,
            x,
        }
    }
}

struct PreReq {
    not: bool,
    delta_i_2d: isize,
    delta_x: isize,
}

impl PreReq {
    const fn none([x, y, z]: [isize; 3]) -> Self {
        Self {
            not: true,
            delta_i_2d: y * I_STRIDE_Y_2D + z * I_STRIDE_Z_2D,
            delta_x: x,
        }
    }

    const fn some([x, y, z]: [isize; 3]) -> Self {
        Self {
            not: false,
            delta_i_2d: y * I_STRIDE_Y_2D + z * I_STRIDE_Z_2D,
            delta_x: x,
        }
    }
}

impl Chunk {
    pub fn liquid_tick(&mut self, tick: u64) {
        let state = FixedState::with_seed(tick);
        let inv_state = FixedState::with_seed(!tick);

        for z in 1..LEN_U32 - 1 {
            'row: for y in 1..LEN_U32 - 1 {
                let i_2d = [y, z].i_2d();

                let mut liquid = self.front_masks.liquid_mask[i_2d] & !PAD_MASK;

                if liquid == 0 {
                    continue 'row;
                }

                {
                    let moved =
                        self.try_move_row(liquid, i_2d, &state, &Delta::new([0, -1, 0]), &[]);

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

                for moves in MOVES {
                    for i in 0..4 {
                        for j in 0..4 {
                            let group = liquid & group_masks[(i + j) % 4];
                            if group == 0 {
                                continue;
                            }

                            let (d, prereqs) = &moves[j];

                            let moved = self.try_move_row(group, i_2d, &state, d, prereqs);

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

    #[inline(always)]
    fn try_move_row(
        &mut self,
        group: u64,
        src_i_2d: usize,
        state: &FixedState,
        d: &Delta,
        prereqs: &[PreReq],
    ) -> u64 {
        let dst_i_2d = src_i_2d.wrapping_add_signed(d.i_2d);

        let prereq_mask = prereqs.iter().fold(!0, |acc, prereq| {
            let i_2d = src_i_2d.wrapping_add_signed(prereq.delta_i_2d);
            let mask = signed_shl(self.front_masks.some_mask[i_2d], prereq.delta_x);

            if prereq.not { acc & !mask } else { acc & mask }
        });

        let try_move = group & !signed_shr(self.front_masks.some_mask[dst_i_2d], d.x) & prereq_mask;

        let dst_some = signed_shr(self.back_masks.some_mask[dst_i_2d], d.x);

        let success = try_move & !dst_some;
        let failure = try_move & dst_some;

        let mut moved = success;

        if success != 0 {
            let add_mask = signed_shl(success, d.x);

            // INVARIANT: Only voxels marked `some` && `liquid` are ever moved with this function.
            self.back_masks.some_mask[dst_i_2d] |= add_mask;
            self.back_masks.liquid_mask[dst_i_2d] |= add_mask;

            self.back_masks.some_mask[src_i_2d] &= !success;
            self.back_masks.liquid_mask[src_i_2d] &= !success;
        }

        for x in BitIter::from(success) {
            let src_i_3d = (x, src_i_2d).i_3d();
            let dst_i_3d = src_i_3d.wrapping_add_signed(d.i_3d);

            self.voxels[dst_i_3d] = self.voxels[src_i_3d];
            self.voxels[src_i_3d] = None;

            self.dst_to_src.insert(dst_i_3d, src_i_3d);
        }

        for x in BitIter::from(failure) {
            let src_i_3d = (x, src_i_2d).i_3d();
            let dst_i_3d = src_i_3d.wrapping_add_signed(d.i_3d);

            let other_src_i_3d = self.dst_to_src.get_mut(&dst_i_3d).unwrap();

            let priority = state.hash_one(src_i_3d);
            let other_priority = state.hash_one(*other_src_i_3d);

            if priority >= other_priority {
                moved |= 1 << x;

                self.back_masks.set(*other_src_i_3d, Some(Voxel::Liquid));
                self.back_masks.set(src_i_3d, None);

                self.voxels[*other_src_i_3d] = self.voxels[src_i_3d];
                self.voxels[src_i_3d] = None;

                *other_src_i_3d = src_i_3d;
            }
        }

        moved
    }
}

fn signed_shl(n: u64, s: isize) -> u64 {
    if s > 0 { n << s } else { n >> -s }
}

fn signed_shr(n: u64, s: isize) -> u64 {
    if s > 0 { n >> s } else { n << -s }
}
