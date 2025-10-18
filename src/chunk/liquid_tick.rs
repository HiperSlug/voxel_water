use rand::random;

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
    // TODO: collide kick up
    pub fn liquid_tick(&mut self) {
        let [read, write] = self.masks.swap_mut();
        *write = read.clone();

        let voxels = &mut self.voxels;

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
                let pz_i = i + STRIDE_Z_2D;
                let nz_i = i - STRIDE_Z_2D;

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

                        let [
                            (rm, add, src_i_2d, dst_i_2d, stride_3d),
                            (k_rm, k_add, k_src_i_2d, k_dst_i_2d, k_stride_3d),
                        ] = match dir {
                            PosX => {
                                let try_fall = (group << 1) & !read.some_mask[ny_i];
                                let fall = try_fall & !write.some_mask[ny_i];
                                let try_kick = try_fall & !fall;
                                let kick = try_kick & !read.some_mask[i] & !write.some_mask[i];

                                const S: isize = I_STRIDE_X_3D - I_STRIDE_Y_3D;
                                const K_S: isize = I_STRIDE_X_3D;
                                [(fall >> 1, fall, i, ny_i, S), (kick >> 1, kick, i, i, K_S)]
                            }
                            NegX => {
                                let try_fall = (group >> 1) & !read.some_mask[ny_i];
                                let fall = try_fall & !write.some_mask[ny_i];
                                let try_kick = try_fall & !fall;
                                let kick = try_kick & !read.some_mask[i] & !write.some_mask[i];

                                const S: isize = -I_STRIDE_X_3D - I_STRIDE_Y_3D;
                                const K_S: isize = -I_STRIDE_X_3D;
                                [(fall << 1, fall, i, ny_i, S), (kick << 1, kick, i, i, K_S)]
                            }
                            PosZ => {
                                let try_fall = group & !read.some_mask[ny_pz_i];
                                let fall = try_fall & !write.some_mask[ny_pz_i];
                                let try_kick = try_fall & !fall;
                                let kick =
                                    try_kick & !read.some_mask[pz_i] & !write.some_mask[pz_i];

                                const S: isize = -I_STRIDE_Y_3D + I_STRIDE_Z_3D;
                                const K_S: isize = I_STRIDE_Z_3D;
                                [(fall, fall, i, ny_pz_i, S), (kick, kick, i, pz_i, K_S)]
                            }
                            NegZ => {
                                let try_fall = group & !read.some_mask[ny_nz_i];
                                let fall = try_fall & !write.some_mask[ny_nz_i];
                                let try_kick = try_fall & !fall;
                                let kick =
                                    try_kick & !read.some_mask[nz_i] & !write.some_mask[nz_i];

                                const S: isize = -I_STRIDE_Y_3D - I_STRIDE_Z_3D;
                                const K_S: isize = -I_STRIDE_Z_3D;
                                [(fall, fall, i, ny_nz_i, S), (kick, kick, i, nz_i, K_S)]
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

                        move_liquid(
                            &mut liquid,
                            write,
                            voxels,
                            k_rm,
                            k_add,
                            k_src_i_2d,
                            k_dst_i_2d,
                            yz_i_3d,
                            k_stride_3d,
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

                        let [
                            (rm, add, src_i_2d, dst_i_2d, stride_3d),
                            (k_rm, k_add, k_src_i_2d, k_dst_i_2d, k_stride_3d),
                        ] = match dir {
                            PosX => {
                                let try_fall = (group << 1) & !read.some_mask[ny_nz_i];
                                let fall = try_fall & !write.some_mask[ny_nz_i];
                                let try_kick = try_fall & !fall;
                                let kick =
                                    try_kick & !read.some_mask[nz_i] & !write.some_mask[nz_i];

                                const S: isize = I_STRIDE_X_3D - I_STRIDE_Y_3D - I_STRIDE_Z_3D;
                                const K_S: isize = I_STRIDE_X_3D - I_STRIDE_Z_3D;
                                [
                                    (fall >> 1, fall, i, ny_nz_i, S),
                                    (kick >> 1, kick, i, nz_i, K_S),
                                ]
                            }
                            NegX => {
                                let try_fall = (group >> 1) & !read.some_mask[ny_pz_i];
                                let fall = try_fall & !write.some_mask[ny_pz_i];
                                let try_kick = try_fall & !fall;
                                let kick =
                                    try_kick & !read.some_mask[pz_i] & !write.some_mask[pz_i];

                                const S: isize = -I_STRIDE_X_3D - I_STRIDE_Y_3D + I_STRIDE_Z_3D;
                                const K_S: isize = -I_STRIDE_X_3D + I_STRIDE_Z_3D;
                                [
                                    (fall << 1, fall, i, ny_pz_i, S),
                                    (kick << 1, kick, i, pz_i, K_S),
                                ]
                            }
                            PosZ => {
                                let try_fall = (group << 1) & !read.some_mask[ny_pz_i];
                                let fall = try_fall & !write.some_mask[ny_pz_i];
                                let try_kick = try_fall & !fall;
                                let kick =
                                    try_kick & !read.some_mask[pz_i] & !write.some_mask[pz_i];

                                const S: isize = I_STRIDE_X_3D - I_STRIDE_Y_3D + I_STRIDE_Z_3D;
                                const K_S: isize = I_STRIDE_X_3D + I_STRIDE_Z_3D;
                                [
                                    (fall >> 1, fall, i, ny_pz_i, S),
                                    (kick >> 1, kick, i, pz_i, K_S),
                                ]
                            }
                            NegZ => {
                                let try_fall = (group >> 1) & !read.some_mask[ny_nz_i];
                                let fall = try_fall & !write.some_mask[ny_nz_i];
                                let try_kick = try_fall & !fall;
                                let kick =
                                    try_kick & !read.some_mask[nz_i] & !write.some_mask[nz_i];

                                const S: isize = -I_STRIDE_X_3D - I_STRIDE_Y_3D - I_STRIDE_Z_3D;
                                const K_S: isize = -I_STRIDE_X_3D - I_STRIDE_Z_3D;
                                [
                                    (fall << 1, fall, i, ny_nz_i, S),
                                    (kick << 1, kick, i, nz_i, K_S),
                                ]
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

                        move_liquid(
                            &mut liquid,
                            write,
                            voxels,
                            k_rm,
                            k_add,
                            k_src_i_2d,
                            k_dst_i_2d,
                            yz_i_3d,
                            k_stride_3d,
                        );

                        if liquid == 0 {
                            continue 'row;
                        }
                    }
                }

                let py_i = i + STRIDE_Y_2D;
                let py_nz_i = py_i - STRIDE_Z_2D;
                let py_pz_i = py_i + STRIDE_Z_2D;

                // adjacent
                for j in 0..4 {
                    for k in 0..4 {
                        let dir = DIRS[k];
                        let group_mask = group_masks[(j + k) % 4];

                        let group = liquid & group_mask;
                        if group == 0 {
                            continue;
                        }

                        let [
                            (rm, add, src_i_2d, dst_i_2d, stride_3d),
                            (k_rm, k_add, k_src_i_2d, k_dst_i_2d, k_stride_3d),
                        ] = match dir {
                            PosX => {
                                let try_slide =
                                    group & !(read.some_mask[i] >> 1) & (read.some_mask[i] << 1);
                                let slide = try_slide & !(write.some_mask[i] >> 1);
                                let try_kick = try_slide & !slide;
                                let kick =
                                    try_kick & !read.some_mask[py_i] & !write.some_mask[py_i];

                                const S: isize = I_STRIDE_X_3D;
                                const K_S: isize = I_STRIDE_X_3D + I_STRIDE_Y_3D;
                                [
                                    (slide, slide << 1, i, i, S),
                                    (kick, kick << 1, i, py_i, K_S),
                                ]
                            }
                            NegX => {
                                let try_slide =
                                    group & !(read.some_mask[i] << 1) & (read.some_mask[i] >> 1);
                                let slide = try_slide & !(write.some_mask[i] << 1);
                                let try_kick = try_slide & !slide;
                                let kick =
                                    try_kick & !read.some_mask[py_i] & !write.some_mask[py_i];

                                const S: isize = -I_STRIDE_X_3D;
                                const K_S: isize = -I_STRIDE_X_3D + I_STRIDE_Y_3D;
                                [
                                    (slide, slide >> 1, i, i, S),
                                    (kick, kick >> 1, i, py_i, K_S),
                                ]
                            }
                            PosZ => {
                                let try_slide =
                                    group & !read.some_mask[pz_i] & read.some_mask[nz_i];
                                let slide = try_slide & !write.some_mask[pz_i];
                                let try_kick = try_slide & !slide;
                                let kick =
                                    try_kick & !read.some_mask[py_pz_i] & !write.some_mask[py_pz_i];

                                const S: isize = I_STRIDE_Z_3D;
                                const K_S: isize = I_STRIDE_Z_3D + I_STRIDE_Y_3D;
                                [(slide, slide, i, pz_i, S), (kick, kick, i, py_pz_i, K_S)]
                            }
                            NegZ => {
                                let try_slide =
                                    group & !read.some_mask[nz_i] & read.some_mask[pz_i];
                                let slide = try_slide & !write.some_mask[nz_i];
                                let try_kick = try_slide & !slide;
                                let kick =
                                    try_kick & !read.some_mask[py_nz_i] & !write.some_mask[py_nz_i];

                                const S: isize = -I_STRIDE_Z_3D;
                                const K_S: isize = -I_STRIDE_Z_3D + I_STRIDE_Y_3D;
                                [(slide, slide, i, nz_i, S), (kick, kick, i, py_nz_i, K_S)]
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

                        move_liquid(
                            &mut liquid,
                            write,
                            voxels,
                            k_rm,
                            k_add,
                            k_src_i_2d,
                            k_dst_i_2d,
                            yz_i_3d,
                            k_stride_3d,
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
