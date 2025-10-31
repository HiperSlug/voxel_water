use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::prelude::*;
use bevy_tweening::{Lens, Tween, TweenAnim, TweeningPlugin};
use rand::{rng, seq::IndexedRandom};
use std::time::Duration;

pub struct JumpscarePlugin;

impl Plugin for JumpscarePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "boo_0.ogg");
        embedded_asset!(app, "boo_1.ogg");
        embedded_asset!(app, "ghost.ktx2");

        app.add_plugins(TweeningPlugin);

        app.add_systems(Startup, init_assets)
            .add_systems(Update, (jumpscare, despawn_ghosts));
    }
}

#[derive(Resource)]
struct Assets {
    boos: Vec<Handle<AudioSource>>,
    ghost: Handle<Image>,
}

fn init_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Assets {
        boos: vec![
            load_embedded_asset!(&*asset_server, "boo_0.ogg"),
            load_embedded_asset!(&*asset_server, "boo_1.ogg"),
        ],
        ghost: load_embedded_asset!(&*asset_server, "ghost.ktx2"),
    });
}

#[derive(Component)]
struct Ghost(Timer);

fn jumpscare(mut commands: Commands, assets: Res<Assets>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::Escape) {
        commands.spawn((
            AudioPlayer(assets.boos.choose(&mut rng()).unwrap().clone()),
            Node {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                width: Val::Px(0.),
                height: Val::Px(0.),
                ..default()
            },
            ImageNode::new(assets.ghost.clone()),
            TweenAnim::new(Tween::new(
                EaseFunction::ElasticOut,
                Duration::from_secs(1),
                UiScaleLens {
                    start: Vec2::ZERO,
                    end: Vec2::splat(350.),
                },
            )),
            Ghost(Timer::from_seconds(1.2, TimerMode::Once)),
        ));
    }
}

fn despawn_ghosts(mut commands: Commands, time: Res<Time>, ghosts: Query<(Entity, &mut Ghost)>) {
    for (entity, mut ghost) in ghosts {
        if ghost.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

struct UiScaleLens {
    start: Vec2,
    end: Vec2,
}

impl Lens<Node> for UiScaleLens {
    fn lerp(&mut self, mut target: Mut<Node>, ratio: f32) {
        target.width = Val::Px(self.start.x + (self.end.x - self.start.x) * ratio);
        target.height = Val::Px(self.start.y + (self.end.y - self.start.y) * ratio);
    }
}
