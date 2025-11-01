pub mod mesher;
pub mod pipeline;

use bevy::{math::U64Vec3, prelude::*};
use bit_iter::BitIter;
use bytemuck::{Pod, Zeroable};
use enum_map::{Enum, EnumMap};

pub use Face::*;

use crate::chunk::index::Index3d;

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
}

#[derive(Component, Deref, DerefMut)]
pub struct ChunkMesh(pub EnumMap<Face, Vec<Quad>>);

impl ChunkMesh {
    #[inline]
    pub fn len(&self) -> usize {
        self.0.values().map(|vec| vec.len()).sum()
    }

    #[inline]
    pub fn quads(&self) -> impl Iterator<Item = &Quad> {
        self.0.values().flat_map(|v| v.iter())
    }
}

#[derive(Component, Default, Clone, Copy)]
pub struct ChunkMeshChanges(pub U64Vec3);

impl ChunkMeshChanges {
    #[inline]
    pub fn clear(&mut self) {
        self.0 = U64Vec3::ZERO
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == U64Vec3::ZERO
    }

    #[inline]
    pub fn push(&mut self, p: impl Index3d) {
        let [x, y, z] = p.xyz();
        self.0.x |= 1 << x;
        self.0.y |= 1 << y;
        self.0.z |= 1 << z;
    }
}
