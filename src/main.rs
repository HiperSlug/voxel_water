// TODO: non-random spreading. Instead base it on neighbor state

mod chunk;
mod double_buffered;
// pub so no dead code
pub mod flycam;
mod mesher;

use std::f32::consts::PI;
use std::time::Duration;

use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::core_pipeline::Skybox;
use bevy::input::mouse::MouseWheel;
use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::prelude::*;

use crate::chunk::{Chunk, Voxel};
use crate::flycam::{FlyCam, NoCameraPlayerPlugin};
use crate::mesher::MESHER;

const MIN_TIMESTEP: Duration = Duration::from_nanos(500_000);
const MAX_TIMESTEP: Duration = Duration::from_secs(2);

const DESTROY_COOLDOWN: Duration = Duration::from_millis(200);

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        NoCameraPlayerPlugin,
        WireframePlugin::default(),
        Game,
    ))
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
    chunk.set_padding(Some(Voxel::Solid));

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
        Mesh3d(
            meshes.add(
                Cuboid::from_length(62.0)
                    .mesh()
                    .build()
                    .with_inverted_winding()
                    .unwrap(),
            ),
        ),
        MeshMaterial3d(materials.add(Color::srgba(1.0, 1.0, 1.0, 0.0))),
        Transform::from_xyz(32.0, 32.0, 32.0),
        Wireframe,
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

fn input(
    mb: Res<ButtonInput<MouseButton>>,
    mut chunk: ResMut<Chunk>,
    transform: Single<&Transform, With<FlyCam>>,
    mut scroll: MessageReader<MouseWheel>,
    mut time_step: ResMut<Time<Fixed>>,
    time: Res<Time>,
    mut mmb_cooldown: Local<Timer>,
    mut rmb_anchor: Local<UVec3>,
) {
    mmb_cooldown.set_duration(DESTROY_COOLDOWN);
    mmb_cooldown.tick(time.delta());

    let [last, dst] = chunk.raycast(Ray3d::new(transform.translation, transform.forward()), 20.0);

    if mb.pressed(MouseButton::Left)
        && let Some(p) = last
    {
        chunk.set(p, Some(Voxel::Liquid));
    }

    if mb.pressed(MouseButton::Middle)
        && mmb_cooldown.is_finished()
        && let Some(p) = dst
    {
        chunk.set(p, None);
        mmb_cooldown.reset();
    }

    if mb.just_pressed(MouseButton::Right)
        && let Some(p) = last.or(dst)
    {
        *rmb_anchor = p;
    } else if mb.just_released(MouseButton::Right)
        && let Some(p) = last.or(dst)
    {
        let min = p.min(*rmb_anchor);
        let max = p.max(*rmb_anchor);
        let max_more = max + UVec3::ONE;
        for z in [min.z, max.z] {
            for y in min.y..max_more.y {
                for x in min.x..max_more.x {
                    chunk.set([x, y, z], Some(Voxel::Solid));
                }
            }
        }
        for z in min.z..max_more.z {
            for x in min.x..max_more.x {
                chunk.set([x, min.y, z], Some(Voxel::Solid));
            }
        }
        for z in min.z..max_more.z {
            for y in min.y..max_more.y { 
                for x in [min.x, max.x] {
                    chunk.set([x, y, z], Some(Voxel::Solid));
                }
            }
        }
    }

    for event in scroll.read() {
        let new = time_step
            .timestep()
            .mul_f64(1.2f64.powf(event.y as f64))
            .clamp(MIN_TIMESTEP, MAX_TIMESTEP);

        time_step.set_timestep(new);
    }
}
