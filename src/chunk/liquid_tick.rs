use bevy::platform::hash::FixedState;
use rand::random;
use std::hash::BuildHasher;

use super::*;

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

impl Chunk {
    // TODO: cells can currently fall through corners
    pub fn liquid_tick(&mut self) {
        let [read, write] = self.masks.swap_mut();
        *write = read.clone();

        let voxels = &mut self.voxels;
        let dst_to_src = &mut self.dst_to_src;
        let tick = self.tick;
        self.tick = tick + 1;

        for z in 1..LEN_U32 - 1 {
            'row: for y in 1..LEN_U32 - 1 {
                let i = linearize_2d([y, z]);
                let yz_i_3d = linearize_3d([0, y, z]);

                let mut liquid = read.liquid_mask[i] & !PAD_MASK;

                if liquid == 0 {
                    continue 'row;
                }

                let ny_i = i - STRIDE_Y_2D;

                // down
                {
                    let fall = liquid & !read.some_mask[ny_i];
                    let success = fall & !write.some_mask[ny_i];
                    let collisions = fall & write.some_mask[ny_i];

                    move_liquid(
                        voxels,
                        write,
                        dst_to_src,
                        success,
                        success,
                        i,
                        ny_i,
                        yz_i_3d,
                        -I_STRIDE_Y_3D,
                    );

                    handle_collisions(
                        voxels,
                        write,
                        dst_to_src,
                        collisions,
                        i,
                        yz_i_3d,
                        -I_STRIDE_Y_3D,
                        tick,
                    );

                    liquid &= !fall;

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
                        let dir = DIRS[k];
                        let group_mask = group_masks[(j + k) % 4];

                        let group = liquid & group_mask;
                        if group == 0 {
                            continue;
                        }

                        let (rm, add, src_i_2d, dst_i_2d, stride_3d, collisions) = match dir {
                            PosX => {
                                let fall = (group << 1) & !read.some_mask[ny_i];
                                let success = fall & !write.some_mask[ny_i];
                                let failure = fall & write.some_mask[ny_i];

                                const S: isize = I_STRIDE_X_3D - I_STRIDE_Y_3D;
                                (success >> 1, success, i, ny_i, S, failure)
                            }
                            NegX => {
                                let fall = (group >> 1) & !read.some_mask[ny_i];
                                let success = fall & !write.some_mask[ny_i];
                                let failure = fall & write.some_mask[ny_i];

                                const S: isize = -I_STRIDE_X_3D - I_STRIDE_Y_3D;
                                (success << 1, success, i, ny_i, S, failure)
                            }
                            PosZ => {
                                let fall = group & !read.some_mask[ny_pz_i];
                                let success = fall & !write.some_mask[ny_pz_i];
                                let failure = fall & write.some_mask[ny_pz_i];

                                const S: isize = -I_STRIDE_Y_3D + I_STRIDE_Z_3D;
                                (success, success, i, ny_pz_i, S, failure)
                            }
                            NegZ => {
                                let fall = group & !read.some_mask[ny_nz_i];
                                let success = fall & !write.some_mask[ny_nz_i];
                                let failure = fall & write.some_mask[ny_nz_i];

                                const S: isize = -I_STRIDE_Y_3D - I_STRIDE_Z_3D;
                                (success, success, i, ny_nz_i, S, failure)
                            }
                        };

                        move_liquid(
                            voxels, write, dst_to_src, rm, add, src_i_2d, dst_i_2d, yz_i_3d,
                            stride_3d,
                        );

                        handle_collisions(
                            voxels, write, dst_to_src, collisions, src_i_2d, yz_i_3d, stride_3d,
                            tick,
                        );

                        if liquid == 0 {
                            continue 'row;
                        }
                    }
                }

                // down-diagonal
                for j in 0..4 {
                    for k in 0..4 {
                        let dir = DIRS[k];
                        let group_mask = group_masks[(j + k) % 4];

                        let group = liquid & group_mask;
                        if group == 0 {
                            continue;
                        }

                        let (rm, add, src_i_2d, dst_i_2d, stride_3d, collisions) = match dir {
                            PosX => {
                                let fall = (group << 1) & !read.some_mask[ny_nz_i];
                                let success = fall & !write.some_mask[ny_nz_i];
                                let failure = fall & write.some_mask[ny_nz_i];

                                const S: isize = I_STRIDE_X_3D - I_STRIDE_Y_3D - I_STRIDE_Z_3D;
                                (success >> 1, success, i, ny_nz_i, S, failure)
                            }
                            NegX => {
                                let fall = (group >> 1) & !read.some_mask[ny_pz_i];
                                let success = fall & !write.some_mask[ny_pz_i];
                                let failure = fall & write.some_mask[ny_pz_i];

                                const S: isize = -I_STRIDE_X_3D - I_STRIDE_Y_3D + I_STRIDE_Z_3D;
                                (success << 1, success, i, ny_pz_i, S, failure)
                            }
                            PosZ => {
                                let fall = (group << 1) & !read.some_mask[ny_pz_i];
                                let success = fall & !write.some_mask[ny_pz_i];
                                let failure = fall & write.some_mask[ny_pz_i];

                                const S: isize = I_STRIDE_X_3D - I_STRIDE_Y_3D + I_STRIDE_Z_3D;
                                (success >> 1, success, i, ny_pz_i, S, failure)
                            }
                            NegZ => {
                                let fall = (group >> 1) & !read.some_mask[ny_nz_i];
                                let success = fall & !write.some_mask[ny_nz_i];
                                let failure = fall & write.some_mask[ny_nz_i];

                                const S: isize = -I_STRIDE_X_3D - I_STRIDE_Y_3D - I_STRIDE_Z_3D;
                                (success << 1, success, i, ny_nz_i, S, failure)
                            }
                        };

                        move_liquid(
                            voxels, write, dst_to_src, rm, add, src_i_2d, dst_i_2d, yz_i_3d,
                            stride_3d,
                        );

                        handle_collisions(
                            voxels, write, dst_to_src, collisions, src_i_2d, yz_i_3d, stride_3d,
                            tick,
                        );

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
                        let dir = DIRS[k];
                        let group_mask = group_masks[(j + k) % 4];

                        let group = liquid & group_mask;
                        if group == 0 {
                            continue;
                        }

                        let (rm, add, src_i_2d, dst_i_2d, stride_3d, collisions) = match dir {
                            PosX => {
                                let slide =
                                    group & !(read.some_mask[i] >> 1) & (read.some_mask[i] << 1);
                                let success = slide & !(write.some_mask[i] >> 1);
                                let failure = slide & (write.some_mask[i] >> 1);

                                const S: isize = I_STRIDE_X_3D;
                                (success, success << 1, i, i, S, failure)
                            }
                            NegX => {
                                let slide =
                                    group & !(read.some_mask[i] << 1) & (read.some_mask[i] >> 1);
                                let success = slide & !(write.some_mask[i] << 1);
                                let failure = slide & (write.some_mask[i] << 1);

                                const S: isize = -I_STRIDE_X_3D;
                                (success, success >> 1, i, i, S, failure)
                            }
                            PosZ => {
                                let slide = group & !read.some_mask[pz_i] & read.some_mask[nz_i];
                                let success = slide & !write.some_mask[pz_i];
                                let failure = slide & write.some_mask[pz_i];

                                const S: isize = I_STRIDE_Z_3D;
                                (success, success, i, pz_i, S, failure)
                            }
                            NegZ => {
                                let slide = group & !read.some_mask[nz_i] & read.some_mask[pz_i];
                                let success = slide & !write.some_mask[nz_i];
                                let failure = slide & write.some_mask[nz_i];

                                const S: isize = -I_STRIDE_Z_3D;
                                (success, success, i, nz_i, S, failure)
                            }
                        };

                        move_liquid(
                            voxels, write, dst_to_src, rm, add, src_i_2d, dst_i_2d, yz_i_3d,
                            stride_3d,
                        );

                        handle_collisions(
                            voxels, write, dst_to_src, collisions, src_i_2d, yz_i_3d, stride_3d,
                            tick,
                        );

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

#[inline]
fn move_liquid(
    voxels: &mut Voxels,
    write: &mut Masks,
    dst_to_src: &mut HashMap<usize, usize>,
    rm: u64,
    add: u64,
    src_i_2d: usize,
    dst_i_2d: usize,
    yz_i_3d: usize,
    stride_3d: isize,
) {
    if rm == 0 {
        return;
    }

    // masks
    write.some_mask[src_i_2d] &= !rm;
    write.liquid_mask[src_i_2d] &= !rm;

    write.some_mask[dst_i_2d] |= add;
    write.liquid_mask[dst_i_2d] |= add;

    // voxels
    let mut moved = rm;
    while moved != 0 {
        let x = moved.trailing_zeros() as usize;
        moved &= moved - 1;

        let src_i_3d = yz_i_3d | x;
        let dst_i_3d = src_i_3d.wrapping_add_signed(stride_3d);

        voxels[dst_i_3d] = voxels[src_i_3d];
        voxels[src_i_3d] = None;

        dst_to_src.insert(dst_i_3d, src_i_3d);
    }
}

#[inline]
fn handle_collisions(
    voxels: &mut Voxels,
    write: &mut Masks,
    dst_to_src: &mut HashMap<usize, usize>,
    mut collisions: u64,
    src_i_2d: usize,
    yz_i_3d: usize,
    stride_3d: isize,
    tick: u64,
) {
    let state = FixedState::with_seed(tick);

    while collisions != 0 {
        let x = collisions.trailing_zeros() as usize;
        collisions &= collisions - 1;

        let src_i_3d = x | yz_i_3d;
        let dst_i_3d = (src_i_3d as isize + stride_3d) as usize;
        let other_src_i_3d = dst_to_src.get_mut(&dst_i_3d).unwrap();

        let other_priority = state.hash_one(*other_src_i_3d);
        let priority = state.hash_one(src_i_3d);

        if priority >= other_priority {
            let other_src_i_2d = *other_src_i_3d >> BITS;
            let other_shift = *other_src_i_3d & ((1 << BITS) - 1);
            write.some_mask[other_src_i_2d] |= 1 << other_shift;
            write.liquid_mask[other_src_i_2d] |= 1 << other_shift;
            write.some_mask[src_i_2d] &= !(1 << x);
            write.liquid_mask[src_i_2d] &= !(1 << x);

            voxels[*other_src_i_3d] = voxels[src_i_3d];
            voxels[src_i_3d] = None;

            *other_src_i_3d = src_i_3d;
        }
    }
}
