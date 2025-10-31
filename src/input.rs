use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use std::time::Duration;

use crate::chunk::{BoxChunk, Voxel};
use crate::flycam::FlyCam;
use crate::render::ChunkMeshChanges;

const MIN_TIMESTEP: Duration = Duration::from_nanos(500_000);
const MAX_TIMESTEP: Duration = Duration::from_secs(2);

pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (chunk_input, time_input));
    }
}

#[derive(Component)]
pub struct SelectedMarker;

fn chunk_input(
    mut transforms: Query<&mut Transform>,
    chunk: Single<(&mut BoxChunk, &mut ChunkMeshChanges)>,
    selected: Single<(Entity, &mut Visibility), With<SelectedMarker>>,
    player: Single<Entity, With<FlyCam>>,
    input: Res<ButtonInput<MouseButton>>,
    mut anchor: Local<UVec3>,
) {
    let (mut chunk, mut changes) = chunk.into_inner();

    let ray = {
        let transform = transforms.get(*player).unwrap();
        let origin = transform.translation;
        let direction = transform.forward();
        Ray3d::new(origin, direction)
    };
    let [prev, dst] = chunk.raycast(ray, 20.);

    let (entity, mut visibility) = selected.into_inner();
    let mut transform = transforms.get_mut(entity).unwrap();

    if input.pressed(MouseButton::Middle)
        && let Some(p) = prev
    {
        chunk.set(p, Some(Voxel::Liquid));
        changes.push(p);
    }

    if input.just_pressed(MouseButton::Left)
        && let Some(p) = dst
    {
        chunk.set(p, None);
        changes.push(p);
    }

    if let Some(p) = prev.or(dst) {
        if input.just_released(MouseButton::Right) {
            let min = p.min(*anchor);
            let max = p.max(*anchor);

            for z in [min.z, max.z] {
                for y in min.y..=max.y {
                    for x in min.x..=max.x {
                        chunk.set([x, y, z], Some(Voxel::Solid));
                        changes.push([x, y, z]);
                    }
                }
            }
            for z in min.z + 1..=max.z - 1 {
                for x in min.x..=max.x {
                    chunk.set([x, min.y, z], Some(Voxel::Solid));
                    changes.push([x, min.y, z]);
                }
            }
            for z in min.z + 1..=max.z - 1 {
                for y in min.y + 1..=max.y {
                    for x in [min.x, max.x] {
                        chunk.set([x, y, z], Some(Voxel::Solid));
                        changes.push([x, y, z]);
                    }
                }
            }
        }

        if !input.pressed(MouseButton::Right) {
            *anchor = p;
        }

        let min = p.min(*anchor);
        let max = p.max(*anchor);

        let scale = (max + UVec3::ONE).as_vec3() - min.as_vec3();
        let translation = min.as_vec3() + scale / 2.;

        transform.scale = scale;
        transform.translation = translation;

        *visibility = Visibility::Visible;
    } else {
        *visibility = Visibility::Hidden;
    }
}

fn time_input(mut time_step: ResMut<Time<Fixed>>, mut scroll: MessageReader<MouseWheel>) {
    for event in scroll.read() {
        let scroll = match event.unit {
            MouseScrollUnit::Line => event.y / 16.,
            MouseScrollUnit::Pixel => event.y,
        };

        #[cfg(not(target_arch = "wasm32"))]
        let scroll = scroll * 8.;
        #[cfg(target_arch = "wasm32")]
        let scroll = scroll / 64.;

        let multiplier = 1.3f64.powf(scroll as f64);

        let new = time_step
            .timestep()
            .mul_f64(multiplier)
            .clamp(MIN_TIMESTEP, MAX_TIMESTEP);
        time_step.set_timestep(new);
    }
}
