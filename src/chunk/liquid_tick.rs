use bevy::platform::hash::FixedState;
use rand::random;
use std::hash::BuildHasher;

use super::*;

const I_STRIDE_Y_2D: isize = STRIDE_Y_2D as isize;
const I_STRIDE_Z_2D: isize = STRIDE_Z_2D as isize;

const I_STRIDE_X_3D: isize = STRIDE_X_3D as isize;
const I_STRIDE_Y_3D: isize = STRIDE_Y_3D as isize;
const I_STRIDE_Z_3D: isize = STRIDE_Z_3D as isize;

const DIRS: [Dir; 4] = [PosX, NegX, PosZ, NegZ];

use Dir::*;
#[derive(Clone, Copy)]
enum Dir {
    PosX,
    NegX,
    PosZ,
    NegZ,
}

impl<'a> FrontMut<'a> {
    #[inline(always)]
    fn try_move_row<const X: isize, const Y: isize, const Z: isize>(
        &mut self,
        dst_to_src: &mut HashMap<usize, usize>,
        try_move: u64,
        src_i_2d: usize,
        yz_i_3d: usize,
        tick: u64,
    ) {
        let state = FixedState::with_seed(tick);

        let delta_2d = Y * I_STRIDE_Y_2D + Z * I_STRIDE_Z_2D;
        let dst_i_2d = src_i_2d.wrapping_add_signed(delta_2d);

        let dst_some = inv_shift::<X>(self.masks.some_mask[dst_i_2d]);

        let mut success = try_move & !dst_some;
        let mut failure = try_move & !success;

        if success != 0 {
            let keep_mask = !success;
            let add_mask = shift::<X>(success);

            // INVARIANT: Only voxels marked `some` && `liquid` are ever moved with this function.
            self.masks.some_mask[dst_i_2d] |= add_mask;
            self.masks.liquid_mask[dst_i_2d] |= add_mask;

            self.masks.some_mask[src_i_2d] &= keep_mask;
            self.masks.liquid_mask[src_i_2d] &= keep_mask;
        }

        while success != 0 {
            let x = success.trailing_zeros() as usize;
            success &= success - 1;

            let src_i_3d = yz_i_3d | x;
            let delta_3d = X * I_STRIDE_X_3D + Y * I_STRIDE_Y_3D + Z * I_STRIDE_Z_3D;
            let dst_i_3d = src_i_3d.wrapping_add_signed(delta_3d);

            self.voxels[dst_i_3d] = self.voxels[src_i_3d];
            self.voxels[src_i_3d] = None;

            dst_to_src.insert(dst_i_3d, src_i_3d);
        }

        while failure != 0 {
            let x = failure.trailing_zeros() as usize;
            failure &= failure - 1;

            let src_i_3d = yz_i_3d | x;
            let delta_3d = X * I_STRIDE_X_3D + Y * I_STRIDE_Y_3D + Z * I_STRIDE_Z_3D;
            let dst_i_3d = src_i_3d.wrapping_add_signed(delta_3d);

            let other_src_i_3d = dst_to_src.get_mut(&dst_i_3d).unwrap();

            let priority = state.hash_one(src_i_3d);
            let other_priority = state.hash_one(*other_src_i_3d);

            if priority >= other_priority {
                self.masks.set(*other_src_i_3d, Some(Voxel::Liquid));
                self.masks.set(src_i_3d, None);

                self.voxels[*other_src_i_3d] = self.voxels[src_i_3d];
                self.voxels[src_i_3d] = None;

                *other_src_i_3d = src_i_3d;
            }
        }
    }
}

impl Chunk {
    // TODO: cells can currently fall through corners
    pub fn liquid_tick(&mut self, tick: u64) {
        let (mut front, read) = self.db_chunk.swap_sync_mut();
        let dst_to_src = &mut self.dst_to_src;

        for z in 1..LEN_U32 - 1 {
            'row: for y in 1..LEN_U32 - 1 {
                let i = [y, z].i_2d();
                let yz_i_3d = i << BITS;

                let mut liquid = read.liquid_mask[i] & !PAD_MASK;

                if liquid == 0 {
                    continue 'row;
                }

                let ny_i = i - STRIDE_Y_2D;

                // down
                {
                    let try_move = liquid & !read.some_mask[ny_i];

                    front.try_move_row::<0, -1, 0>(dst_to_src, try_move, i, yz_i_3d, tick);

                    liquid &= !try_move;

                    if liquid == 0 {
                        continue 'row;
                    }
                }

                let ny_pz_i = ny_i + STRIDE_Z_2D;
                let ny_nz_i = ny_i - STRIDE_Z_2D;

                // random groups
                let x_mask = random::<u64>();
                let pos_mask = random::<u64>();

                let group_masks = [
                    x_mask & pos_mask,
                    x_mask & !pos_mask,
                    !x_mask & pos_mask,
                    !x_mask & !pos_mask,
                ];

                // down-adjacent
                for j in 0..4 {
                    for k in 0..4 {
                        let group = liquid & group_masks[(j + k) % 4];
                        if group == 0 {
                            continue;
                        }

                        match DIRS[k] {
                            PosX => {
                                let try_move = group & inv_shift::<1>(!read.some_mask[ny_i]);
                                liquid &= !try_move;
                                front.try_move_row::<1, -1, 0>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                            NegX => {
                                let try_move = group & inv_shift::<-1>(!read.some_mask[ny_i]);
                                liquid &= !try_move;
                                front.try_move_row::<-1, -1, 0>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                            PosZ => {
                                let try_move = group & !read.some_mask[ny_pz_i];
                                liquid &= !try_move;
                                front.try_move_row::<0, -1, 1>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                            NegZ => {
                                let try_move = group & !read.some_mask[ny_nz_i];
                                liquid &= !try_move;
                                front.try_move_row::<0, -1, -1>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                        };

                        if liquid == 0 {
                            continue 'row;
                        }
                    }
                }

                // down-diagonal
                for j in 0..4 {
                    for k in 0..4 {
                        let group = liquid & group_masks[(j + k) % 4];
                        if group == 0 {
                            continue;
                        }

                        match DIRS[k] {
                            PosX => {
                                let try_move = group & inv_shift::<1>(!read.some_mask[ny_nz_i]);
                                liquid &= !try_move;
                                front.try_move_row::<1, -1, -1>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                            NegX => {
                                let try_move = group & inv_shift::<-1>(!read.some_mask[ny_pz_i]);
                                liquid &= !try_move;
                                front.try_move_row::<-1, -1, 1>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                            PosZ => {
                                let try_move = group & inv_shift::<1>(!read.some_mask[ny_pz_i]);
                                liquid &= !try_move;
                                front.try_move_row::<1, -1, 1>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                            NegZ => {
                                let try_move = group & inv_shift::<-1>(!read.some_mask[ny_nz_i]);
                                liquid &= !try_move;
                                front.try_move_row::<-1, -1, -1>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                        };

                        if liquid == 0 {
                            continue 'row;
                        }
                    }
                }

                let pz_i = i + STRIDE_Z_2D;
                let nz_i = i - STRIDE_Z_2D;

                // adjacent
                for j in 0..4 {
                    for k in 0..4 {
                        let group = liquid & group_masks[(j + k) % 4];
                        if group == 0 {
                            continue;
                        }

                        match DIRS[k] {
                            PosX => {
                                let try_move = group
                                    & inv_shift::<1>(!read.some_mask[i])
                                    & shift::<1>(read.some_mask[i]);
                                liquid &= !try_move;
                                front.try_move_row::<1, 0, 0>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                            NegX => {
                                let try_move = group
                                    & inv_shift::<-1>(!read.some_mask[i])
                                    & shift::<-1>(read.some_mask[i]);
                                liquid &= !try_move;
                                front.try_move_row::<-1, 0, 0>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                            PosZ => {
                                let try_move = group & !read.some_mask[pz_i] & read.some_mask[nz_i];
                                liquid &= !try_move;
                                front.try_move_row::<0, 0, 1>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                            NegZ => {
                                let try_move = group & !read.some_mask[nz_i] & read.some_mask[pz_i];
                                liquid &= !try_move;
                                front.try_move_row::<0, 0, -1>(
                                    dst_to_src, try_move, i, yz_i_3d, tick,
                                );
                            }
                        };

                        if liquid == 0 {
                            continue 'row;
                        }
                    }
                }
            }
        }
        self.dst_to_src.clear();
    }
}

#[inline(always)]
fn shift<const S: isize>(n: u64) -> u64 {
    if S > 0 { n << S } else { n >> -S }
}

#[inline(always)]
fn inv_shift<const S: isize>(n: u64) -> u64 {
    if S > 0 { n >> S } else { n << -S }
}
