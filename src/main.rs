mod chunk;
// pub so no dead code
pub mod flycam;
mod mesher;

use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::prelude::*;

use crate::chunk::Chunk;
use crate::flycam::PlayerPlugin;
use crate::mesher::Mesher;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PlayerPlugin, WireframePlugin::default()))
        .init_resource::<Chunk>()
        .init_resource::<Mesher>()
        .add_systems(Startup, setup)
        .add_systems(Update, greedy_mesh_render.run_if(resource_changed::<Chunk>))
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
) {
    let mesh = mesh_assets.add(Rectangle::from_length(1.0));
    let material = material_assets.add(Color::WHITE);
    commands.insert_resource(Handles {
        mesh: mesh.clone(),
        material: material.clone(),
    });

    commands.spawn(PointLight::default());
    commands.spawn((Mesh3d(mesh_assets.add(Sphere::default())), MeshMaterial3d(material)));
}

fn greedy_mesh_render(
    mut commands: Commands,
    chunk: Res<Chunk>,
    mut mesher: ResMut<Mesher>,
    handles: Res<Handles>,
    mut last: Query<(Entity, &mut Transform), With<QuadMarker>>,
) {
    mesher.mesh(&chunk);

    let mut quad_iter = mesher.quads.iter().map(|quad| quad.rectangle_transform());
    let mut last_iter = last.iter_mut();

    for ((_, mut transform), new) in (&mut last_iter).zip(&mut quad_iter) {
        *transform = new;
    }

    for (entity, _) in last_iter {
        commands.entity(entity).despawn();
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
}
