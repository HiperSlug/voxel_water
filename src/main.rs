mod block;
mod chunk;
mod flycam;
mod input;
mod jumpscare;
mod render;

use std::f32::consts::PI;

use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::core_pipeline::Skybox;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;

use crate::chunk::{BoxChunk, Voxel};
use crate::flycam::{FlyCam, NoCameraPlayerPlugin};
use crate::input::{GameInputPlugin, SelectedMarker};
use crate::jumpscare::JumpscarePlugin;
use crate::render::mesher::MESHER;
use crate::render::pipeline::QuadInstancingPlugin;
use crate::render::{ChunkMesh, ChunkMeshChanges};

fn main() {
    App::new().add_plugins(Game).run();
}

struct Game;

impl Plugin for Game {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins,
            NoCameraPlayerPlugin,
            GameInputPlugin,
            QuadInstancingPlugin,
            JumpscarePlugin,
        ));

        embedded_asset!(app, "skybox.ktx2");

        app.insert_resource(Time::<Fixed>::from_hz(10.0));

        app.add_systems(Startup, setup)
            .add_systems(FixedUpdate, liquid_tick)
            .add_systems(Update, remesh_chunk);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_at(
            Vec3::NEG_Y
                .rotate_towards(Vec3::Z, PI / 5.)
                .rotate_towards(Vec3::X, PI / 10.),
            Vec3::Y,
        ),
    ));

    // player
    commands.spawn((
        Transform {
            translation: vec3(32.0, 80.0, -8.0),
            rotation: Quat::from_rotation_x(PI / 4.) * Quat::from_rotation_y(PI),
            ..default()
        },
        Skybox {
            image: load_embedded_asset!(&*asset_server, "skybox.ktx2"),
            brightness: 1000.0,
            ..default()
        },
        Camera3d::default(),
        FlyCam,
        NoIndirectDrawing, // TODO: what does this do?
    ));

    // chunk aabb
    commands.spawn((
        Mesh3d(meshes.add(cube_wireframe_mesh(62.))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(32.0, 32.0, 32.0),
    ));

    // selected aabb
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_length(1.))),
        MeshMaterial3d(materials.add(Color::srgba(0.5, 0., 0., 0.25))),
        Visibility::Hidden,
        SelectedMarker,
    ));

    // chunk
    let mut chunk = BoxChunk::default();
    chunk.fill_padding(Some(Voxel::Solid));
    let mesh = MESHER.with_borrow_mut(|mesher| mesher.mesh(&chunk, IVec3::ZERO));
    commands.spawn((
        chunk,
        mesh,
        ChunkMeshChanges::default(),
        Mesh3d(meshes.add(Rectangle::from_length(1.))),
        NoFrustumCulling,
    ));
}

fn liquid_tick(chunk: Single<(&mut BoxChunk, &mut ChunkMeshChanges)>, mut tick: Local<u64>) {
    let (mut chunk, mut changes) = chunk.into_inner();

    chunk.liquid_tick(*tick);
    chunk.masks.dblt_masks.copy_back_to_front();

    for (dst, src) in chunk.dst_to_src.drain() {
        changes.push(dst);
        changes.push(src);
    }

    *tick += 1;
}

fn remesh_chunk(chunk: Single<(&BoxChunk, &mut ChunkMesh, &mut ChunkMeshChanges)>) {
    let (chunk, mut mesh, mut changes) = chunk.into_inner();

    if changes.is_empty() {
        return;
    }

    MESHER.with_borrow_mut(|mesher| {
        mesher.remesh(chunk, IVec3::ZERO, &mut mesh, *changes);
    });

    changes.clear();
}

// AI
pub fn cube_wireframe_mesh(size: f32) -> Mesh {
    let h = size / 2.;

    let corners = [
        vec3(-h, -h, -h),
        vec3(h, -h, -h),
        vec3(h, h, -h),
        vec3(-h, h, -h),
        vec3(-h, -h, h),
        vec3(h, -h, h),
        vec3(h, h, h),
        vec3(-h, h, h),
    ];
    let edges = [
        [0, 1],
        [1, 2],
        [2, 3],
        [3, 0],
        [4, 5],
        [5, 6],
        [6, 7],
        [7, 4],
        [0, 4],
        [1, 5],
        [2, 6],
        [3, 7],
    ];

    let positions = corners.iter().map(|c| c.to_array()).collect::<Vec<_>>();
    let indices = edges.into_iter().flatten().collect::<Vec<_>>();

    let mut mesh = Mesh::new(PrimitiveTopology::LineList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
