// TODO: non-random spreading. Instead base it on neighbor state

mod chunk;
// pub so no dead code
pub mod flycam;
mod mesher;
mod water;

use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::prelude::*;

use crate::chunk::Chunk;
use crate::flycam::PlayerPlugin;
use crate::mesher::MESHER;
use crate::water::DoubleBuffered;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PlayerPlugin, WireframePlugin::default()))
        .init_resource::<DoubleBuffered>()
        .add_systems(Startup, setup)
        .insert_resource(Time::<Fixed>::from_hz(120.0))
        .add_systems(FixedUpdate, tick)
        .add_systems(Update, greedy_mesh_render) //.run_if(resource_changed::<Chunk>))
        .run();
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
    mut chunk: ResMut<DoubleBuffered>,
) {
    let mesh = mesh_assets.add(Rectangle::from_length(1.0));
    let material = material_assets.add(Color::WHITE);
    commands.insert_resource(Handles {
        mesh: mesh.clone(),
        material: material.clone(),
    });

    *chunk.front_mut() = Chunk::nz_init();
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
