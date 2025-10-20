use std::array;
use std::hash::{BuildHasher, Hasher, RandomState};
use rand::{random, rng, prelude::*};

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

        // TODO: a deterministic solution to handling priority.
        let mut zs: [_; LEN - 2] = array::from_fn(|i| i as u32 + 1);
        let mut ys: [_; LEN - 2] = array::from_fn(|i| i as u32 + 1);
        
        zs.shuffle(&mut rng());
        for z in zs {
            ys.shuffle(&mut rng());
            'row: for y in ys {
                let i = linearize_2d([y, z]);
                let yz_i_3d = linearize_3d([0, y, z]);

                let mut liquid = read.liquid_mask[i] & !PAD_MASK;

                if liquid == 0 {
                    continue 'row;
                }

                let ny_i = i - STRIDE_Y_2D;

                // down
                {
                    let fall = liquid & !read.some_mask[ny_i] & !write.some_mask[ny_i];

                    move_liquid(
                        &mut liquid,
                        write,
                        voxels,
                        fall,
                        fall,
                        i,
                        ny_i,
                        yz_i_3d,
                        -I_STRIDE_Y_3D,
                    );

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

                        let (rm, add, src_i_2d, dst_i_2d, stride_3d) = match dir {
                            PosX => {
                                let fall = (group << 1) & !read.some_mask[ny_i] & !write.some_mask[ny_i];

                                const S: isize = I_STRIDE_X_3D - I_STRIDE_Y_3D;
                                (fall >> 1, fall, i, ny_i, S)
                            }
                            NegX => {
                                let fall = (group >> 1) & !read.some_mask[ny_i] & !write.some_mask[ny_i];

                                const S: isize = -I_STRIDE_X_3D - I_STRIDE_Y_3D;
                                (fall << 1, fall, i, ny_i, S)
                            }
                            PosZ => {
                                let fall = group & !read.some_mask[ny_pz_i] & !write.some_mask[ny_pz_i];

                                const S: isize = -I_STRIDE_Y_3D + I_STRIDE_Z_3D;
                                (fall, fall, i, ny_pz_i, S)
                            }
                            NegZ => {
                                let fall = group & !read.some_mask[ny_nz_i] & !write.some_mask[ny_nz_i];

                                const S: isize = -I_STRIDE_Y_3D - I_STRIDE_Z_3D;
                                (fall, fall, i, ny_nz_i, S)
                            }
                        };

                        move_liquid(
                            &mut liquid,
                            write,
                            voxels,
                            rm,
                            add,
                            src_i_2d,
                            dst_i_2d,
                            yz_i_3d,
                            stride_3d,
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

                        let (rm, add, src_i_2d, dst_i_2d, stride_3d) = match dir {
                            PosX => {
                                let fall = (group << 1) & !read.some_mask[ny_nz_i] & !write.some_mask[ny_nz_i];

                                const S: isize = I_STRIDE_X_3D - I_STRIDE_Y_3D - I_STRIDE_Z_3D;
                                (fall >> 1, fall, i, ny_nz_i, S)
                            }
                            NegX => {
                                let fall = (group >> 1) & !read.some_mask[ny_pz_i] & !write.some_mask[ny_pz_i];

                                const S: isize = -I_STRIDE_X_3D - I_STRIDE_Y_3D + I_STRIDE_Z_3D;
                                (fall << 1, fall, i, ny_pz_i, S)
                            }
                            PosZ => {
                                let fall = (group << 1) & !read.some_mask[ny_pz_i] & !write.some_mask[ny_pz_i];

                                const S: isize = I_STRIDE_X_3D - I_STRIDE_Y_3D + I_STRIDE_Z_3D;
                                (fall >> 1, fall, i, ny_pz_i, S)
                            }
                            NegZ => {
                                let fall = (group >> 1) & !read.some_mask[ny_nz_i] & !write.some_mask[ny_nz_i];

                                const S: isize = -I_STRIDE_X_3D - I_STRIDE_Y_3D - I_STRIDE_Z_3D;
                                (fall << 1, fall, i, ny_nz_i, S)
                            }
                        };

                        move_liquid(
                            &mut liquid,
                            write,
                            voxels,
                            rm,
                            add,
                            src_i_2d,
                            dst_i_2d,
                            yz_i_3d,
                            stride_3d,
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

                        let (rm, add, src_i_2d, dst_i_2d, stride_3d) = match dir {
                            PosX => {
                                let slide = group & !(read.some_mask[i] >> 1) & (read.some_mask[i] << 1) & !(write.some_mask[i] >> 1);

                                const S: isize = I_STRIDE_X_3D;
                                (slide, slide << 1, i, i, S)
                            }
                            NegX => {
                                let slide = group & !(read.some_mask[i] << 1) & (read.some_mask[i] >> 1) & !(write.some_mask[i] << 1);

                                const S: isize = -I_STRIDE_X_3D;
                                (slide, slide >> 1, i, i, S)
                            }
                            PosZ => {
                                let slide = group & !read.some_mask[pz_i] & read.some_mask[nz_i] & !write.some_mask[pz_i];

                                const S: isize = I_STRIDE_Z_3D;
                                (slide, slide, i, pz_i, S)
                            }
                            NegZ => {
                                let slide = group & !read.some_mask[nz_i] & read.some_mask[pz_i] & !write.some_mask[nz_i];

                                const S: isize = -I_STRIDE_Z_3D;
                                (slide, slide, i, nz_i, S)
                            }
                        };

                        move_liquid(
                            &mut liquid,
                            write,
                            voxels,
                            rm,
                            add,
                            src_i_2d,
                            dst_i_2d,
                            yz_i_3d,
                            stride_3d,
                        );

                        if liquid == 0 {
                            continue 'row;
                        }
                    }
                }
            }
        }
    }
}

#[inline]
fn move_liquid(
    liquid: &mut u64,
    write: &mut Masks,
    voxels: &mut Voxels,
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
    *liquid &= !rm;
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
    }
}
