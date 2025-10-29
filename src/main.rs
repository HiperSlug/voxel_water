mod chunk;
mod flycam;
mod jumpscare;
mod render;

use std::f32::consts::PI;
use std::time::Duration;

use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::core_pipeline::Skybox;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;

use crate::chunk::{Chunk, Voxel};
use crate::flycam::{FlyCam, NoCameraPlayerPlugin};
use crate::jumpscare::JumpscarePlugin;
use crate::render::ChunkMesh;
use crate::render::mesher::MESHER;
use crate::render::pipeline::{ChunkQuads, QuadInstancingPlugin};

const MIN_TIMESTEP: Duration = Duration::from_nanos(500_000);
const MAX_TIMESTEP: Duration = Duration::from_secs(2);

fn main() {
    App::new().add_plugins(Game).run();
}

struct Game;

impl Plugin for Game {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins,
            NoCameraPlayerPlugin,
            QuadInstancingPlugin,
            JumpscarePlugin,
        ));

        embedded_asset!(app, "skybox.ktx2");

        app.insert_resource(Time::<Fixed>::from_hz(10.0));

        app.add_systems(Startup, setup)
            .add_systems(FixedUpdate, liquid_tick)
            .add_systems(Update, (render_chunk, rotate_skybox, input));
    }
}

#[derive(Component, DerefMut, Deref, Default)]
struct BoxChunk(Box<Chunk>);

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
            translation: vec3(30.0, 85.0, -10.0),
            rotation: Quat::from_array([0.0, 0.8, 0.5, 0.0]).normalize(),
            ..default()
        },
        Skybox {
            image: load_embedded_asset!(&*asset_server, "skybox.ktx2"),
            brightness: 1000.0,
            ..default()
        },
        Camera3d::default(),
        FlyCam,
        NoIndirectDrawing, // do I need this
    ));

    // chunk aabb
    commands.spawn((
        Mesh3d(meshes.add(cuboid_wireframe_mesh(Vec3::splat(62.0)))),
        MeshMaterial3d(materials.add(Color::srgba(1.0, 1.0, 1.0, 1.0))),
        Transform::from_xyz(32.0, 32.0, 32.0),
    ));

    // selected aabb
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        MeshMaterial3d(materials.add(Color::srgba(0.5, 0., 0., 0.5))),
        Transform::default(),
        Visibility::Hidden,
        SelectedMarker,
    ));

    // chunk
    let quad = Rectangle::from_length(1.0)
        .mesh()
        .build()
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_COLOR,
            vec![Color::WHITE.to_srgba().to_vec4(); 4],
        );
    let mut chunk = BoxChunk::default();
    chunk.fill_padding(Some(Voxel::Solid));
    let mesh = MESHER.with_borrow_mut(|mesher| mesher.mesh(&chunk, IVec3::ZERO));
    commands.spawn((
        Mesh3d(meshes.add(quad)),
        mesh,
        chunk,
        ChunkQuads::default(),
        NoFrustumCulling,
    ));
}

fn rotate_skybox(time: Res<Time>, mut skybox: Single<&mut Skybox>) {
    const ANGULAR_VEL: f32 = -0.003;
    let delta = ANGULAR_VEL * time.delta_secs();
    skybox.rotation *= Quat::from_rotation_y(delta);
}

fn liquid_tick(chunk: Single<(&mut BoxChunk, &mut ChunkMesh)>, mut tick: Local<u64>) {
    let (mut chunk, mut mesh) = chunk.into_inner();

    chunk.liquid_tick(*tick);
    *tick += 1;

    let Chunk {
        dst_to_src,
        front_masks,
        back_masks,
        ..
    } = &mut ***chunk; // lol
    *front_masks = back_masks.clone();

    for (dst, src) in dst_to_src.drain() {
        // so this actually might be worse because there is no per-row dedup
        // per-row dedup would require either some cool alogrithm (i cannot think of one), or another collection (dense = 64^2 bits, sparse = hashset)
        // let (_, i_2d) = dst.x_and_i_2d();
        // let (_, i_2d_2) = src.x_and_i_2d();

        // front_masks.some_mask[i_2d] = back_masks.some_mask[i_2d];
        // front_masks.liquid_mask[i_2d] = back_masks.liquid_mask[i_2d];

        // front_masks.some_mask[i_2d_2] = back_masks.some_mask[i_2d_2];
        // front_masks.liquid_mask[i_2d_2] = back_masks.liquid_mask[i_2d_2];

        mesh.push_change(dst);
        mesh.push_change(src);
    }

    chunk.dst_to_src.clear();
}

fn render_chunk(chunk: Single<(&BoxChunk, &mut ChunkMesh, &mut ChunkQuads)>) {
    MESHER.with_borrow_mut(|mesher| {
        let (chunk, mut mesh, mut quads) = chunk.into_inner();

        mesher.remesh(chunk, IVec3::ZERO, &mut mesh);

        quads.clear();
        for vec in mesh.values() {
            quads.extend(vec);
        }
    })
}

#[derive(Component)]
struct SelectedMarker;

fn input(
    mut transforms: Query<&mut Transform>,
    selected_q: Single<(Entity, &mut Visibility), With<SelectedMarker>>,
    player_q: Single<Entity, With<FlyCam>>,
    mb: Res<ButtonInput<MouseButton>>,
    chunk: Single<(&mut BoxChunk, &mut ChunkMesh)>,
    mut scroll: MessageReader<MouseWheel>,
    mut time_step: ResMut<Time<Fixed>>,
    mut anchor: Local<UVec3>,
    chunk_quads: Single<&ChunkQuads>,
) {
    let (mut chunk, mut mesh) = chunk.into_inner();
    let transform = transforms.get(*player_q).unwrap();

    if mb.just_pressed(MouseButton::Back) {
        println!("{:?}", chunk_quads[0]);
    }

    let [last, dst] = chunk.raycast(Ray3d::new(transform.translation, transform.forward()), 20.0);

    let (selected_entity, mut selected_visibility) = selected_q.into_inner();
    let mut selected_transform = transforms.get_mut(selected_entity).unwrap();

    if mb.pressed(MouseButton::Middle)
        && let Some(p) = last
    {
        chunk.set(p, Some(Voxel::Liquid));
        mesh.push_change(p);
    }

    if mb.just_pressed(MouseButton::Left)
        && let Some(p) = dst
    {
        chunk.set(p, None);
        mesh.push_change(p);
    }

    if let Some(p) = last.or(dst) {
        if mb.just_released(MouseButton::Right) {
            let min = p.min(*anchor);
            let max = p.max(*anchor);

            for z in [min.z, max.z] {
                for y in min.y..=max.y {
                    for x in min.x..=max.x {
                        chunk.set([x, y, z], Some(Voxel::Solid));
                        mesh.push_change([x, y, z]);
                    }
                }
            }
            for z in min.z + 1..=max.z - 1 {
                for x in min.x..=max.x {
                    chunk.set([x, min.y, z], Some(Voxel::Solid));
                    mesh.push_change([x, min.y, z]);
                }
            }
            for z in min.z + 1..=max.z - 1 {
                for y in min.y + 1..=max.y {
                    for x in [min.x, max.x] {
                        chunk.set([x, y, z], Some(Voxel::Solid));
                        mesh.push_change([x, y, z]);
                    }
                }
            }
        }

        if !mb.pressed(MouseButton::Right) {
            *anchor = p;
        }

        let min = p.min(*anchor);
        let max = p.max(*anchor);

        let scale = (max + UVec3::ONE).as_vec3() - min.as_vec3();
        let translation = min.as_vec3() + scale / 2.;

        selected_transform.scale = scale;
        selected_transform.translation = translation;

        *selected_visibility = Visibility::Visible;
    } else {
        *selected_visibility = Visibility::Hidden;
    }

    for event in scroll.read() {
        #[cfg(not(target_arch = "wasm32"))]
        let scroll = match event.unit {
            MouseScrollUnit::Line => event.y / 16.,
            MouseScrollUnit::Pixel => event.y,
        } * 8.;
        #[cfg(target_arch = "wasm32")]
        let scroll = match event.unit {
            MouseScrollUnit::Line => event.y / 16.,
            MouseScrollUnit::Pixel => event.y,
        } / 100.;

        let new = time_step
            .timestep()
            .mul_f64(1.2f64.powf(scroll as f64))
            .clamp(MIN_TIMESTEP, MAX_TIMESTEP);

        time_step.set_timestep(new);
    }
}

// AI
pub fn cuboid_wireframe_mesh(size: Vec3) -> Mesh {
    let h = size / 2.;

    let corners = [
        vec3(-h.x, -h.y, -h.z),
        vec3(h.x, -h.y, -h.z),
        vec3(h.x, h.y, -h.z),
        vec3(-h.x, h.y, -h.z),
        vec3(-h.x, -h.y, h.z),
        vec3(h.x, -h.y, h.z),
        vec3(h.x, h.y, h.z),
        vec3(-h.x, h.y, h.z),
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
