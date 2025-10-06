mod flycam;

use bevy::{
    pbr::wireframe::{Wireframe, WireframePlugin},
    prelude::*,
    render::RenderDebugFlags,
};
use ndshape::{ConstPow2Shape2u32, ConstPow2Shape3u32, ConstShape as _};
use std::iter;

use crate::flycam::PlayerPlugin;

const BITS: u32 = 6;
const LEN: usize = 1 << BITS; // 64
const AREA: usize = LEN * LEN;
const VOL: usize = LEN * LEN * LEN;

type VolShape = ConstPow2Shape3u32<BITS, BITS, BITS>;
type AreaShape = ConstPow2Shape2u32<BITS, BITS>;

#[derive(Resource)]
struct Chunk {
    some_masks: [u64; AREA],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            some_masks: [u64::MAX; AREA],
        }
    }
}

impl Chunk {
    fn iter_some(&self) -> impl Iterator<Item = UVec3> {
        self.some_masks
            .iter()
            .enumerate()
            .flat_map(|(i, some_mask)| {
                let [y, z] = AreaShape::delinearize(i as u32);
                let mut some_mask = *some_mask;
                iter::from_fn(move || {
                    if some_mask != 0 {
                        let x = some_mask.trailing_zeros();
                        some_mask &= some_mask - 1;
                        Some(uvec3(x, y, z))
                    } else {
                        None
                    }
                })
            })
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PlayerPlugin, WireframePlugin::default()))
        .init_resource::<Chunk>()
        .add_systems(Startup, setup)
        .add_systems(Update, naive_render)
        .run();
}

#[derive(Resource)]
struct Handles {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct CuboidMarker;

fn setup(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Handles {
        mesh: mesh_assets.add(Cuboid::from_length(1.0)),
        material: material_assets.add(Color::WHITE),
    });
}

fn naive_render(
    mut commands: Commands,
    chunk: Res<Chunk>,
    handles: Res<Handles>,
    mut last: Query<(Entity, &mut Transform), With<CuboidMarker>>,
) {
    let mut iter_some = chunk.iter_some();
    let mut iter_last = last.iter_mut();

    for ((_, mut transform), translation) in (&mut iter_last).zip(&mut iter_some) {
        transform.translation = translation.as_vec3();
    }

    for (entity, _) in iter_last {
        commands.entity(entity).despawn();
    }

    for pos in iter_some {
        commands.spawn((
            Transform::from_translation(pos.as_vec3()),
            Mesh3d(handles.mesh.clone()),
            MeshMaterial3d(handles.material.clone()),
            CuboidMarker,
            Wireframe,
        ));
    }
}
