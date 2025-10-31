// use bevy::{
//     asset::{embedded_asset, load_embedded_asset}, image::ImageLoaderSettings, prelude::*, render::render_resource::{Extent3d, TextureDimension, TextureFormat}, tasks::{block_on, poll_once}
// };

// pub struct TextureArrayPlugin;

// impl Plugin for TextureArrayPlugin {
//     fn build(&self, app: &mut App) {
//         embedded_asset!(app, "not_water.png");
//         embedded_asset!(app, "water.png");

//         app.add_systems(Startup, init_handles)
//             .add_systems(PostStartup, build);
//     }
// }

// #[derive(Resource)]
// struct Handles(Vec<Handle<Image>>);

// #[derive(Resource)]
// struct TextureArray(Handle<Image>);

// fn init_handles(mut commands: Commands, asset_server: Res<AssetServer>) {
//     let handles = vec![
//         load_embedded_asset!(&*asset_server, "not_water.png"),
//         load_embedded_asset!(&*asset_server, "water.png"),
//     ];
//     commands.insert_resource(Handles(handles));
// }

// fn build(mut commands: Commands, handles: Res<Handles>, mut image_assets: ResMut<Assets<Image>>) {
//     let layers = handles.0.len() as u32;
//     let size = image_assets.get(&handles.0[0]).unwrap().size();

//     let data = handles
//         .0
//         .iter()
//         .flat_map(|id| image_assets.get(id).unwrap().data.as_ref().unwrap())
//         .copied()
//         .collect::<Vec<_>>();

//     ImageLoaderSettings

//     let texture_array = Image::new(
//         Extent3d {
//             width: size.x,
//             height: size.y,
//             depth_or_array_layers: layers,
//         },
//         TextureDimension::D2,
//         data,
//         TextureFormat::bevy_default(),
//         default(),
//     );

//     let handle = image_assets.add(texture_array);

//     commands.remove_resource::<Handles>();
//     commands.insert_resource(TextureArray(handle));
// }
