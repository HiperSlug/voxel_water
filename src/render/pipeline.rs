use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    core_pipeline::core_3d::Transparent3d,
    ecs::system::{
        SystemParamItem,
        lifetimeless::{Read, SRes},
    },
    mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout, VertexFormat},
    pbr::{
        MeshPipeline, MeshPipelineKey, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup,
        SetMeshViewBindingArrayBindGroup,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{RenderMesh, RenderMeshBufferInfo, allocator::MeshAllocator},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{
            AsBindGroup, BindGroup, BindGroupLayout, Buffer, BufferInitDescriptor, BufferUsages,
            PipelineCache, RenderPipelineDescriptor, SpecializedMeshPipeline,
            SpecializedMeshPipelineError, SpecializedMeshPipelines, VertexAttribute,
            VertexStepMode,
        },
        renderer::RenderDevice,
        storage::GpuShaderStorageBuffer,
        sync_world::MainEntity,
        texture::{FallbackImage, GpuImage},
        view::ExtractedView,
    },
};

use super::Quad;

pub struct QuadInstancingPlugin;

impl Plugin for QuadInstancingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "quad.wgsl");

        app.add_plugins((
            ExtractComponentPlugin::<ChunkQuads>::default(),
            ExtractComponentPlugin::<ArrayTextureMaterial>::default(),
        ));

        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawCustom>()
            .init_resource::<SpecializedMeshPipelines<CustomPipeline>>()
            .add_systems(RenderStartup, init_custom_pipeline)
            .add_systems(
                Render,
                (
                    queue_quads.in_set(RenderSystems::QueueMeshes),
                    prepare_instance_buffers.in_set(RenderSystems::PrepareResources),
                    prepare_bind_group.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

#[derive(Resource)]
struct CustomPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    layout: BindGroupLayout,
}

fn init_custom_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
) {
    commands.insert_resource(CustomPipeline {
        shader: load_embedded_asset!(&*asset_server, "quad.wgsl"),
        mesh_pipeline: mesh_pipeline.clone(),
        layout: ArrayTextureMaterial::bind_group_layout(&render_device),
    });
}

impl SpecializedMeshPipeline for CustomPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;

        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: size_of::<Quad>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Sint32x3,
                    offset: 0,
                    shader_location: 8,
                },
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: VertexFormat::Sint32x3.size(),
                    shader_location: 9,
                },
            ],
        });
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.layout.push(self.layout.clone());
        Ok(descriptor)
    }
}

// go between to get the quad data to the render world
#[derive(Component, ExtractComponent, Clone, Deref, DerefMut, Default)]
pub struct ChunkQuads(Vec<Quad>);

fn queue_quads(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CustomPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    material_meshes: Query<(Entity, &MainEntity), With<ChunkQuads>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(&ExtractedView, &Msaa)>,
) {
    let draw_custom = transparent_3d_draw_functions.read().id::<DrawCustom>();

    for (view, msaa) in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();
        for (entity, main_entity) in &material_meshes {
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*main_entity)
            else {
                continue;
            };
            let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let key =
                view_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());

            let pipeline = pipelines
                .specialize(&pipeline_cache, &custom_pipeline, key, &mesh.layout)
                .unwrap();
            transparent_phase.add(Transparent3d {
                entity: (entity, *main_entity),
                pipeline,
                draw_function: draw_custom,
                distance: rangefinder.distance_translation(&mesh_instance.translation),
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

#[derive(Component)]
struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &ChunkQuads)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, quads) in &query {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(quads),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: quads.len(),
        });
    }
}

#[derive(Component, ExtractComponent, AsBindGroup, Debug, Clone)]
pub struct ArrayTextureMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub array_texture: Handle<Image>,
}

// fn edit_image(
//     image_assets: ResMut<Assets<Image>>,
//     query: Query<&ArrayTextureMaterial>,
// ) {
//     for mat in query {
//         let Some(image) = image_assets.get_mut(mat.array_texture) else {
//             continue;
//         };
//         let desc = image.sampler.get_or_init_descriptor();
//         desc.address_mode_u = ImageAddressMode::Repeat;
//         desc.address_mode_v = ImageAddressMode::Repeat;
//     }
// }

fn prepare_bind_group(
    mut commands: Commands,
    query: Single<(Entity, &ArrayTextureMaterial)>,
    render_device: Res<RenderDevice>,
    pipeline: Res<CustomPipeline>,

    gpu_images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
    gpu_shader_storage_buffer: Res<RenderAssets<GpuShaderStorageBuffer>>,
) {
    let (entity, material) = query.into_inner();
    let bind_group = material
        .as_bind_group(
            &pipeline.layout,
            &render_device,
            &mut (gpu_images, fallback_image, gpu_shader_storage_buffer),
        )
        .ok()
        .map(|b| b.bind_group);

    commands
        .entity(entity)
        .insert(TextureArrayBindGroup(bind_group));
}

#[derive(Component, Debug)]
struct TextureArrayBindGroup(Option<BindGroup>);

type DrawCustom = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    // SetMaterialBindGroup<3>, // this was returning RenderCommandResult::Skip
    DrawMeshInstanced,
);

struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMeshInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = (Read<TextureArrayBindGroup>, Read<InstanceBuffer>);

    #[inline]
    fn render<'w>(
        // It seems this function is never getting called.
        item: &P,
        _view: (),
        item_q: Option<(&'w TextureArrayBindGroup, &'w InstanceBuffer)>,
        (meshes, render_mesh_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // A borrow check workaround.
        let mesh_allocator = mesh_allocator.into_inner();
        // info!("marker1");

        let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        // info!("marker1.1");
        let Some(gpu_mesh) = meshes.into_inner().get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };

        // info!("marker1.2");

        let Some((bind_group, instance_buffer)) = item_q else {
            return RenderCommandResult::Skip;
        };
        // info!("marker2 {}", instance_buffer.length);

        if instance_buffer.length == 0 {
            return RenderCommandResult::Skip;
        }
        // info!("marker2.1 {bind_group:?}");
        let Some(bind_group) = &bind_group.0 else {
            return RenderCommandResult::Skip;
        };
        // info!("marker3"); // UNREACHED

        let Some(vertex_buffer_slice) =
            mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id)
        else {
            return RenderCommandResult::Skip;
        };

        // info!("marker4");

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        pass.set_bind_group(3, bind_group, &[]);

        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) =
                    mesh_allocator.mesh_index_slice(&mesh_instance.mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);
                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    0..instance_buffer.length as u32,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, 0..instance_buffer.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
