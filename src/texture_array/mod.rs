use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    image::ImageAddressMode,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

pub struct TextureArrayPlugin;

impl Plugin for TextureArrayPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "claws.png");
        embedded_asset!(app, "glass.png");

        app.add_systems(Startup, init_handles)
            .add_systems(Update, texture_array);
    }
}

#[derive(Resource, Deref)]
struct TextureArrayHandles(Vec<Handle<Image>>);

fn init_handles(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(TextureArrayHandles(vec![
        load_embedded_asset!(&*asset_server, "claws.png"),
        load_embedded_asset!(&*asset_server, "glass.png"),
    ]));
}

#[derive(Resource, Deref)]
pub struct TextureArrayHandle(pub Handle<Image>);

fn texture_array(
    mut commands: Commands,
    handles: If<Res<TextureArrayHandles>>,
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
            width: 48,
            height: 48,
            depth_or_array_layers: handles.len() as u32,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );

    let desc = image.sampler.get_or_init_descriptor();
    desc.address_mode_u = ImageAddressMode::Repeat;
    desc.address_mode_v = ImageAddressMode::Repeat;

    let handle = image_assets.add(image);

    commands.remove_resource::<TextureArrayHandles>();
    commands.insert_resource(TextureArrayHandle(handle));
}
