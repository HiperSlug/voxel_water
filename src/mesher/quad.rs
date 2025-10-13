use bevy::{prelude::*, render::render_resource::ShaderType};
use bytemuck::{Pod, Zeroable};
use std::mem::transmute;

use super::Face::{self, *};

const MASK6: u32 = (1 << 6) - 1;
const MASK3: u32 = (1 << 3) - 1;

const MAX6: u32 = MASK6;
const MAX16: u32 = u16::MAX as u32;

const WIDTH_SHIFT: u32 = 0;
const HEIGHT_SHIFT: u32 = 6;
const FACE_SHIFT: u32 = 12;
// unused bit 15
const TEXTURE_SHIFT: u32 = 16;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Quad {
    pub pos: IVec3,
    other: u32,
}

impl Quad {
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

    pub fn width(&self) -> u32 {
        (self.other >> WIDTH_SHIFT) & MASK6
    }

    pub fn height(&self) -> u32 {
        (self.other >> HEIGHT_SHIFT) & MASK6
    }

    pub fn size(&self) -> UVec2 {
        uvec2(self.width(), self.height())
    }

    pub fn face(&self) -> Face {
        let num = (self.other >> FACE_SHIFT) & MASK3;
        // SAFETY: only constructed with `new`
        unsafe { transmute(num) }
    }

    pub fn texture(&self) -> u32 {
        self.other >> TEXTURE_SHIFT
    }

    pub fn rectangle_transform(&self) -> Transform {
        let mut translation = self.pos.as_vec3();
        let size = self.size().as_vec2();
        let half_size = size / 2.;
        let scale = size.extend(1.);

        let (delta_translation, rotation) = match self.face() {
            PosX => (
                vec3(1.0, half_size.y, half_size.x),
                Quat::from_rotation_arc(Vec3::Z, Vec3::X),
            ),
            NegX => (
                vec3(0.0, half_size.y, half_size.x),
                Quat::from_rotation_arc(Vec3::Z, Vec3::NEG_X),
            ),
            PosY => (
                vec3(half_size.x, 1.0, half_size.y),
                Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
            ),
            NegY => (
                vec3(half_size.x, 0.0, half_size.y),
                Quat::from_rotation_arc(Vec3::Z, Vec3::NEG_Y),
            ),
            PosZ => (
                vec3(half_size.x, half_size.y, 1.0),
                Quat::from_rotation_arc(Vec3::Z, Vec3::Z),
            ),
            NegZ => (
                vec3(half_size.x, half_size.y, 0.0),
                Quat::from_rotation_arc(Vec3::Z, Vec3::NEG_Z),
            ),
        };

        translation += delta_translation;

        Transform {
            translation,
            rotation,
            scale,
        }
    }
}
