pub mod mesher;
pub mod pipeline;

use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};
use enum_map::{Enum, EnumMap};

pub use Face::*;

const MAX6: u32 = (1 << 6) - 1;
const MAX16: u32 = u16::MAX as u32;

const WIDTH_SHIFT: u32 = 0;
const HEIGHT_SHIFT: u32 = 6;
const FACE_SHIFT: u32 = 12;
// skip 1
const TEXTURE_SHIFT: u32 = 16;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Quad {
    pub pos: IVec3,
    other: u32,
}

impl Quad {
    #[inline]
    pub fn new(pos: IVec3, w: u32, h: u32, f: Face, t: u32) -> Self {
        debug_assert!(w <= MAX6, "width: {w} > {MAX6}");
        debug_assert!(h <= MAX6, "height: {h} > {MAX6}");
        debug_assert!(t <= MAX16, "texture: {t} > {MAX16}");

        let f = f as u32;

        Self {
            pos,
            other: (w << WIDTH_SHIFT)
                | (h << HEIGHT_SHIFT)
                | (f << FACE_SHIFT)
                | (t << TEXTURE_SHIFT),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Enum)]
pub enum Face {
    PosX = 0,
    PosY = 1,
    PosZ = 2,
    NegX = 3,
    NegY = 4,
    NegZ = 5,
}

impl Face {
    const ALL: [Self; 6] = [PosX, PosY, PosZ, NegX, NegY, NegZ];

    fn axis(self) -> Axis {
        match self {
            PosX | NegX => Axis::X,
            PosY | NegY => Axis::Y,
            PosZ | NegZ => Axis::Z,
        }
    }
}

enum Axis {
    X,
    Y,
    Z,
}

pub struct ChunkMesh {
    map: EnumMap<Face, Vec<Quad>>,
}
