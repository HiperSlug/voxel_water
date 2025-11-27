use bevy::asset::{embedded_asset, load_embedded_asset};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDimension,
};

pub struct SkyboxImagePlugin;

impl Plugin for SkyboxImagePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "nx.png");
        embedded_asset!(app, "px.png");
        embedded_asset!(app, "ny.png");
        embedded_asset!(app, "py.png");
        embedded_asset!(app, "nz.png");
        embedded_asset!(app, "pz.png");

        app.add_systems(Startup, init_handles)
            .add_systems(Update, cubemap);
    }
}

#[derive(Resource, Deref)]
struct SkyboxHandles([Handle<Image>; 6]);

fn init_handles(mut commands: Commands, asset_server: Res<AssetServer>) {
    // order matters
    commands.insert_resource(SkyboxHandles([
        load_embedded_asset!(&*asset_server, "px.png"),
        load_embedded_asset!(&*asset_server, "nx.png"),
        load_embedded_asset!(&*asset_server, "py.png"),
        load_embedded_asset!(&*asset_server, "ny.png"),
        load_embedded_asset!(&*asset_server, "pz.png"),
        load_embedded_asset!(&*asset_server, "nz.png"),
    ]));
}

#[derive(Resource, Deref)]
pub struct SkyboxHandle(pub Handle<Image>);

fn cubemap(
    mut commands: Commands,
    handles: If<Res<SkyboxHandles>>,
    mut image_assets: ResMut<Assets<Image>>,
) {
    if !handles.iter().all(|id| image_assets.contains(id)) {
        return;
    }

    let data = handles
        .iter()
        .flat_map(|id| image_assets.get(id).unwrap().data.as_ref().unwrap())
        .copied()
        .collect();

    let mut image = Image::new(
        Extent3d {
            width: 2048,
            height: 2048,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );

    let desc = image.texture_view_descriptor.get_or_insert_default();
    desc.dimension = Some(TextureViewDimension::Cube);

    let handle = image_assets.add(image);

    commands.remove_resource::<SkyboxHandles>();
    commands.insert_resource(SkyboxHandle(handle));
}
