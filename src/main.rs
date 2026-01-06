mod flycam;
mod input;
mod liquid_tick;
mod render;
mod skybox;
mod texture_array;
mod voxels;

use std::f32::consts::PI;

// use bevy::camera::visibility::NoFrustumCulling;
use bevy::core_pipeline::Skybox;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;
use dashmap::DashMap;

use crate::flycam::{FlyCam, NoCameraPlayerPlugin};
use crate::input::{GameInputPlugin, SelectedMarker};
use crate::render::mesher::MESHER;
use crate::render::pipeline::{QuadInstancingPlugin, TextureArrayMaterial};
use crate::render::{ChunkMesh, ChunkRemesh};
use crate::skybox::{SkyboxHandle, SkyboxImagePlugin};
use crate::texture_array::{TextureArrayHandle, TextureArrayPlugin};

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        NoCameraPlayerPlugin,
        GameInputPlugin,
        QuadInstancingPlugin,
        SkyboxImagePlugin,
        TextureArrayPlugin,
    ));

    app.insert_resource(Time::<Fixed>::from_hz(10.0))
        .init_state::<GameState>()
        .add_systems(Update, setup.run_if(in_state(GameState::Setup)));
        // .add_systems(
        //     FixedUpdate,
        //     liquid_tick.run_if(in_state(GameState::NotSetup)),
        // )
        // .add_systems(Update, remesh_chunk.run_if(in_state(GameState::NotSetup)));

    app.run();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, States, Default)]
enum GameState {
    #[default]
    Setup,
    NotSetup,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    // conditional
    skybox_handle: If<Res<SkyboxHandle>>,
    texture_array_handle: If<Res<TextureArrayHandle>>,
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
            image: skybox_handle.clone(),
            brightness: 1000.0,
            ..default()
        },
        // Msaa::Off,
        Camera3d::default(),
        FlyCam,
        NoIndirectDrawing, // TODO: what does this do?
    ));

    // // chunk aabb
    // commands.spawn((
    //     Mesh3d(meshes.add(cube_wireframe_mesh(62.))),
    //     MeshMaterial3d(materials.add(Color::WHITE)),
    //     Transform::from_xyz(32.0, 32.0, 32.0),
    // ));

    // selected aabb
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_length(1.))),
        MeshMaterial3d(materials.add(Color::srgba(0.5, 0., 0., 0.25))),
        Visibility::Hidden,
        SelectedMarker,
    ));

    // chunk
    // let mut chunks = SparseChunks::default();
    // let mut chunk_meshes = SparseChunkMeshes::default();

    // chunks.fill(ivec3(0, 0, 0), ivec3(0, 0, 0), Some(DIRT));
    // chunks.fill_padding(DIRT);
    // let mesh = MESHER.with_borrow_mut(|mesher| mesher.mesh(&chunk, IVec3::ZERO));

    // commands.spawn((
    //     // data
    //     chunks,
    //     chunk_meshes,
    //     SparseChunkRemeshes::default(),
    //     // rendering
    //     Mesh3d(meshes.add(Rectangle::from_length(1.))),
    //     NoFrustumCulling,
    //     TextureArrayMaterial {
    //         handle: texture_array_handle.clone(),
    //     },
    // ));

    // commands.remove_resource::<SkyboxHandle>();
    // commands.remove_resource::<TextureArrayHandle>();
    // commands.set_state(GameState::NotSetup);
}

// fn liquid_tick(
//     chunk: Single<(&mut BoxChunk, &mut ChunkRemesh)>,
//     mut tick: Local<u64>,
//     dst_to_src: Local<DashMap<usize, usize>>,
// ) {
//     let (mut chunk, mut changes) = chunk.into_inner();

//     chunk.collect_moves(&dst_to_src, *tick);

//     for k_v in dst_to_src.iter() {
//         let (&dst, &src) = k_v.pair();

//         chunk.transfer(dst, src);

//         changes.push(dst);
//         changes.push(src);
//     }

//     dst_to_src.clear();

//     *tick += 1;
// }

// fn remesh_chunk(chunk: Single<(&BoxChunk, &mut ChunkMesh, &mut ChunkRemesh)>) {
//     let (chunk, mut mesh, mut changes) = chunk.into_inner();

//     if changes.is_empty() {
//         return;
//     }

//     MESHER.with_borrow_mut(|mesher| {
//         mesher.remesh(chunk, IVec3::ZERO, &mut mesh, *changes);
//     });

//     changes.clear();
// }

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
