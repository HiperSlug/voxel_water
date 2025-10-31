#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}

const MASK6: u32 = (1 << 6) - 1;
const MASK3: u32 = (1 << 3) - 1;

const WIDTH_SHIFT: u32 = 0;
const HEIGHT_SHIFT: u32 = 6;
const FACE_SHIFT: u32 = 12;
// skip 1
const TEXTURE_SHIFT: u32 = 16;

fn instance_width(other: u32) -> u32 {
    return (other >> WIDTH_SHIFT) & MASK6;
}

fn instance_height(other: u32) -> u32 {
    return (other >> HEIGHT_SHIFT) & MASK6;
}

fn instance_size(other: u32) -> vec2<u32> {
    return vec2<u32>(instance_width(other), instance_height(other));
}

fn instance_face(other: u32) -> u32 {
    return (other >> FACE_SHIFT) & MASK3;
}

fn instance_texture(other: u32) -> u32 {
    return other >> TEXTURE_SHIFT;
}

const POS_X: u32 = 0;
const POS_Y: u32 = 1;
const POS_Z: u32 = 2;
const NEG_X: u32 = 3;
const NEG_Y: u32 = 4;
// const NEG_Z: u32 = 5;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    @location(8) instance_position: vec3<i32>,
    @location(9) instance_other: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(5) color: vec4<f32>,

    @location(8) @interpolate(flat) texture: u32,
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let instance_size = instance_size(vertex.instance_other);
    let instance_face = instance_face(vertex.instance_other);
    let instance_texture = instance_texture(vertex.instance_other);
    
    let p = vertex.position;
    let n = vertex.normal;
    let s = vec2<f32>(instance_size);
    let h = s / 2.0;

    var position: vec3<f32>;

    var out: VertexOutput;

    out.uv = vertex.uv * s;
    out.texture = instance_texture;

    switch(instance_face) {
        case POS_X: {
            position = vec3(1.0, p.y * s.y + h.y, -p.x * s.x + h.x);
            out.world_normal = vec3(n.z, n.y, -n.x);
        }
        case NEG_X: {
            position = vec3(0.0, p.y * s.y + h.y, p.x * s.x + h.x);
            out.world_normal = vec3(-n.z, n.y, n.x);
        }
        case POS_Y: {
            position = vec3(p.x * s.x + h.x, 1.0, -p.y * s.y + h.y);
            out.world_normal = vec3(n.x, n.z, -n.y);
        }
        case NEG_Y: {
            position = vec3(p.x * s.x + h.x, 0.0, p.y * s.y + h.y);
            out.world_normal = vec3(n.x, -n.z, n.y);
        }
        case POS_Z: {
            position = vec3(p.x * s.x + h.x, p.y * s.y + h.y, 1.0);
            out.world_normal = vec3(n.x, n.y, n.z);
        }
        case default: { // && NEG_Z
            position = vec3(-p.x * s.x + h.x, p.y * s.y + h.y, 0.0);
            out.world_normal = vec3(-n.x, n.y, -n.z);
        }
    }
    position += vec3<f32>(vertex.instance_position);

    let world_position = vec4(position, 1.0);

    var identity: mat4x4<f32> = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );

    // TODO: workaround through uniform
    out.world_position = world_position;
    out.position = mesh_position_local_to_clip(
        identity,
        // get_world_from_local(0u),
        world_position
    );

    // TODO: lighting
    switch(instance_face) {
        case POS_X: {
            out.color = vec4(vec3(0.9), 1.0);
        }
        case NEG_X: {
            out.color = vec4(vec3(0.1), 1.0);
        }
        case POS_Y: {
            out.color = vec4(vec3(0.8), 1.0);
        }
        case NEG_Y: {
            out.color = vec4(vec3(0.2), 1.0);
        }
        case POS_Z: {
            out.color = vec4(vec3(0.7), 1.0);
        }
        case default: { // && NEG_Z
            out.color = vec4(vec3(0.3), 1.0);
        }
    }

    return out;
}

#import bevy_pbr::{
    mesh_view_bindings::view,
    pbr_types::{PbrInput, pbr_input_new},
    pbr_functions as fns,
    pbr_bindings,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(3) @binding(0) var my_array_texture: texture_2d_array<f32>;
@group(3) @binding(1) var my_array_texture_sampler: sampler;

@fragment
fn fragment(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();

    pbr_input.material.base_color = textureSample(my_array_texture, my_array_texture_sampler, in.uv, in.texture);
    pbr_input.material.base_color *= in.color;

    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = in.world_normal;

    pbr_input.is_orthographic = view.clip_from_view[3][3] == 1.0;

    pbr_input.N = normalize(pbr_input.world_normal);

    pbr_input.V = fns::calculate_view(in.world_position, pbr_input.is_orthographic);

    return tone_mapping(fns::apply_pbr_lighting(pbr_input), view.color_grading);
}
