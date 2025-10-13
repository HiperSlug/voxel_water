mod chunk;
// pub so no dead code
pub mod flycam;
mod mesher;

use std::f32::consts::PI;
use std::time::Duration;

use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::core_pipeline::Skybox;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::chunk::{Chunk, Voxel};
use crate::flycam::{FlyCam, NoCameraPlayerPlugin};
use crate::mesher::MESHER;

const MIN_TIMESTEP: Duration = Duration::from_nanos(500_000);
const MAX_TIMESTEP: Duration = Duration::from_secs(2);

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, NoCameraPlayerPlugin, Game))
        .add_systems(Startup, setup)
        .run();
}

struct Game;

impl Plugin for Game {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "skybox.ktx2");

        app.insert_resource(Time::<Fixed>::from_hz(10.0))
            .init_resource::<Chunk>();

        app.add_systems(Startup, init_quad_handles)
            .add_systems(FixedUpdate, liquid_tick)
            .add_systems(Update, (render_chunk, rotate_skybox, input));
    }
}

#[derive(Resource)]
struct QuadHandles {
    quad: Handle<Mesh>,
    liquid: Handle<StandardMaterial>,
    solid: Handle<StandardMaterial>,
}

fn init_quad_handles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(QuadHandles {
        quad: meshes.add(Rectangle::from_length(1.0)),
        liquid: materials.add(Color::srgb_u8(235, 244, 250)),
        solid: materials.add(Color::srgb_u8(235, 244, 250).darker(0.7)),
    });
}

#[derive(Component)]
struct QuadMarker;

fn setup(
    mut commands: Commands,
    mut chunk: ResMut<Chunk>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: ResMut<AssetServer>,
) {
    chunk.fill_padding(Some(Voxel::Solid));

    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_at(
            Vec3::NEG_Y
                .rotate_towards(Vec3::Z, PI / 5.5)
                .rotate_towards(Vec3::X, PI / 10.5),
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
}

fn rotate_skybox(time: Res<Time>, mut skybox: Single<&mut Skybox>) {
    const ANGULAR_VEL: f32 = -0.003;
    let delta = ANGULAR_VEL * time.delta_secs();
    skybox.rotation *= Quat::from_rotation_y(delta);
}

fn liquid_tick(mut chunk: ResMut<Chunk>) {
    chunk.liquid_tick();
}

fn render_chunk(
    mut commands: Commands,
    chunk: Res<Chunk>,
    handles: Res<QuadHandles>,
    mut old_quads: Query<
        (
            &mut Visibility,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<QuadMarker>,
    >,
) {
    MESHER.with_borrow_mut(|mesher| {
        let quads = mesher.mesh(&chunk);

        let mut new_iter = quads.iter();
        let mut old_iter = old_quads.iter_mut();

        for ((mut visibility, mut transform, mut material), quad) in
            (&mut old_iter).zip(&mut new_iter)
        {
            *transform = quad.rectangle_transform();
            *visibility = Visibility::Visible;
            material.0 = match quad.voxel {
                Voxel::Liquid => handles.liquid.clone(),
                Voxel::Solid => handles.solid.clone(),
            };
        }

        for (mut visibility, _, _) in old_iter {
            *visibility = Visibility::Hidden;
        }

        for quad in new_iter {
            commands.spawn((
                quad.rectangle_transform(),
                Mesh3d(handles.quad.clone()),
                MeshMaterial3d(match quad.voxel {
                    Voxel::Liquid => handles.liquid.clone(),
                    Voxel::Solid => handles.solid.clone(),
                }),
                QuadMarker,
            ));
        }
    })
}

#[derive(Component)]
struct SelectedMarker;

fn input(
    mut transforms: Query<&mut Transform>,
    selected_q: Single<(Entity, &mut Visibility), With<SelectedMarker>>,
    mb: Res<ButtonInput<MouseButton>>,
    mut chunk: ResMut<Chunk>,
    player_q: Single<Entity, With<FlyCam>>,
    mut scroll: MessageReader<MouseWheel>,
    mut time_step: ResMut<Time<Fixed>>,
    mut anchor: Local<UVec3>,
) {
    let transform = transforms.get(*player_q).unwrap();

    let [last, dst] = chunk.raycast(Ray3d::new(transform.translation, transform.forward()), 20.0);

    let (selected_entity, mut selected_visibility) = selected_q.into_inner();
    let mut selected_transform = transforms.get_mut(selected_entity).unwrap();

    if mb.pressed(MouseButton::Middle)
        && let Some(p) = last
    {
        chunk.set(p, Some(Voxel::Liquid));
    }

    if mb.just_pressed(MouseButton::Left)
        && let Some(p) = dst
    {
        chunk.set(p, None);
    }

    if let Some(p) = last.or(dst) {
        if mb.just_released(MouseButton::Right) {
            let min = p.min(*anchor);
            let max = p.max(*anchor);

            for z in [min.z, max.z] {
                for y in min.y..=max.y {
                    for x in min.x..=max.x {
                        chunk.set([x, y, z], Some(Voxel::Solid));
                    }
                }
            }
            for z in min.z + 1..=max.z - 1 {
                for x in min.x..=max.x {
                    chunk.set([x, min.y, z], Some(Voxel::Solid));
                }
            }
            for z in min.z + 1..=max.z - 1 {
                for y in min.y + 1..=max.y {
                    for x in [min.x, max.x] {
                        chunk.set([x, y, z], Some(Voxel::Solid));
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
