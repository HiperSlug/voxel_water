use bevy::prelude::*;

use crate::chunk::Voxel;

use super::Face::{self, *};

#[derive(Debug, Clone, Copy)]
pub struct Quad {
    pub pos: UVec3,
    pub size: UVec2,
    pub face: Face,
    // TODO: replace with texture
    pub voxel: Voxel,
}

impl Quad {
    pub fn rectangle_transform(&self) -> Transform {
        let mut translation = self.pos.as_vec3();
        let size = self.size.as_vec2();
        let half_size = size / 2.;
        let scale = size.extend(1.);

        let (delta_translation, rotation) = match self.face {
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
