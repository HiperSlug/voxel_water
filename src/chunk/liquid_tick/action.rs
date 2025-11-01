use super::super::index::*;

const I_STRIDE_Y_2D: isize = STRIDE_Y_2D as isize;
const I_STRIDE_Z_2D: isize = STRIDE_Z_2D as isize;

const I_STRIDE_X_3D: isize = STRIDE_X_3D as isize;
const I_STRIDE_Y_3D: isize = STRIDE_Y_3D as isize;
const I_STRIDE_Z_3D: isize = STRIDE_Z_3D as isize;

pub type Action = (Delta, &'static [PreReq]);
type ActionGroup = [Action; 4];

pub struct Delta([isize; 3]);

impl Delta {
    #[inline]
    pub fn i_3d(&self) -> isize {
        let [x, y, z] = self.0;
        x * I_STRIDE_X_3D + y * I_STRIDE_Y_3D + z * I_STRIDE_Z_3D
    }

    #[inline]
    pub fn x_and_i_2d(&self) -> (isize, isize) {
        let [x, y, z] = self.0;
        (x, y * I_STRIDE_Y_2D + z * I_STRIDE_Z_2D)
    }
}

pub struct PreReq {
    pub not: bool,
    pub delta: Delta,
}

const fn none(delta: [isize; 3]) -> PreReq {
    PreReq {
        not: true,
        delta: Delta(delta),
    }
}

const fn some(delta: [isize; 3]) -> PreReq {
    PreReq {
        not: false,
        delta: Delta(delta),
    }
}

const fn action(delta: [isize; 3], prereqs: &'static [PreReq]) -> Action {
    (Delta(delta), prereqs)
}

pub const DOWN_ACTION: Action = action([0, -1, 0], &[]);

pub const ACTIONS: &[ActionGroup] = &[
    [
        action([1, -1, 0], &[none([1, 0, 0])]),
        action([-1, -1, 0], &[none([-1, 0, 0])]),
        action([0, -1, 1], &[none([0, 0, 1])]),
        action([0, -1, -1], &[none([0, 0, -1])]),
    ],
    [
        action(
            [1, -1, 1],
            &[none([1, 0, 0]), none([0, 0, 1]), none([1, 0, 1])],
        ),
        action(
            [-1, -1, 1],
            &[none([-1, 0, 0]), none([0, 0, 1]), none([-1, 0, 1])],
        ),
        action(
            [1, -1, -1],
            &[none([1, 0, 0]), none([0, 0, -1]), none([1, 0, -1])],
        ),
        action(
            [-1, -1, -1],
            &[none([-1, 0, 0]), none([0, 0, -1]), none([-1, 0, -1])],
        ),
    ],
    [
        action([1, 0, 0], &[some([-1, 0, 0])]),
        action([-1, 0, 0], &[some([1, 0, 0])]),
        action([0, 0, 1], &[some([0, 0, -1])]),
        action([0, 0, -1], &[some([0, 0, 1])]),
    ],
];
