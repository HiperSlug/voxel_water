// TODO: non-random spreading. Instead base it on neighbor state

mod chunk;
// pub so no dead code
pub mod flycam;
mod mesher;
mod water;

use std::f32::consts::PI;

use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::core_pipeline::Skybox;
use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;

use crate::chunk::Chunk;
use crate::flycam::{FlyCam, NoCameraPlayerPlugin};
use crate::mesher::MESHER;
use crate::water::DoubleBuffered;

fn main() {
    let mut app = App::new();
    app
        .add_plugins((DefaultPlugins, NoCameraPlayerPlugin))
        // .add_plugins(bevy::pbr::wireframe::WireframePlugin::default())
        .init_resource::<DoubleBuffered>()
        .add_systems(Startup, setup)
        .insert_resource(Time::<Fixed>::from_hz(30.0))
        .add_systems(FixedUpdate, tick)
        .add_systems(Update, (greedy_mesh_render, rotate_skybox));

    embedded_asset!(app, "cubemap.ktx2");

    app.run();
}

#[derive(Resource)]
struct Handles {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct QuadMarker;

fn setup(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    asset_server: ResMut<AssetServer>,
    mut chunk: ResMut<DoubleBuffered>,
) {
    let mesh = mesh_assets.add(Rectangle::from_length(1.0));
    let material = material_assets.add(Color::srgb_u8(235, 244, 250));
    commands.insert_resource(Handles {
        mesh: mesh.clone(),
        material: material.clone(),
    });

    *chunk.front_mut() = Chunk::nz_init();

    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_at(
            Vec3::NEG_Y
                .rotate_towards(Vec3::Z, PI / 5.5)
                .rotate_towards(Vec3::X, PI / 10.5),
            Vec3::Y,
        ),
    ));

    let image = load_embedded_asset!(&*asset_server, "cubemap.ktx2");

    commands.spawn((
        Transform {
            translation: vec3(30.0, 85.0, -10.0),
            rotation: Quat::from_array([0.0, 0.8, 0.5, 0.0]).normalize(),
            ..default()
        },
        Camera3d::default(),
        Skybox {
            image,
            brightness: 1000.0,
            ..default()
        },
        FlyCam,
    ));

    commands.insert_resource(AmbientLight {
        color: Color::srgb_u8(210, 220, 240),
        brightness: 1.0,
        ..default()
    });
}

fn rotate_skybox(time: Res<Time>, mut skybox: Single<&mut Skybox>) {
    const ANGULAR_VEL: f32 = -0.005;
    let delta = ANGULAR_VEL * time.delta_secs();
    skybox.rotation *= Quat::from_rotation_y(delta);
}


fn tick(mut chunk: ResMut<DoubleBuffered>) {
    chunk.tick();
}

fn greedy_mesh_render(
    mut commands: Commands,
    chunk: Res<DoubleBuffered>,
    handles: Res<Handles>,
    mut last: Query<(&mut Visibility, &mut Transform), With<QuadMarker>>,
) {
    MESHER.with_borrow_mut(|mesher| {
        let quads = mesher.mesh(chunk.front());

        let mut quad_iter = quads.iter().map(|quad| quad.rectangle_transform());
        let mut last_iter = last.iter_mut();

        for ((mut visibility, mut transform), new) in (&mut last_iter).zip(&mut quad_iter) {
            *transform = new;
            *visibility = Visibility::Visible;
        }

        for (mut visibility, _) in last_iter {
            *visibility = Visibility::Hidden;
        }

        for transform in quad_iter {
            commands.spawn((
                transform,
                Mesh3d(handles.mesh.clone()),
                MeshMaterial3d(handles.material.clone()),
                QuadMarker,
                Wireframe,
            ));
        }
    })
}
