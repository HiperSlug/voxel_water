#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_functions::get_world_from_local

const MASK6: u32 = (1 << 6) - 1;
const MASK3: u32 = (1 << 3) - 1;

const WIDTH_SHIFT: u32 = 0;
const HEIGHT_SHIFT: u32 = 6;
const FACE_SHIFT: u32 = 12;
// skip 1
const TEXTURE_SHIFT: u32 = 16;

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

    @location(3) i_position: vec3<i32>,
    @location(4) i_other: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let custom_model_matrix = custom_model_matrix(
        vertex.i_position,
        i_size(vertex.i_other),
        i_face(vertex.i_other),
    );

    let model_matrix = get_world_from_local(0u);
    let p = custom_model_matrix * model_matrix * vec4<f32>(vertex.position, 1.0);
    let clip_position = position_world_to_clip(p.xyz);

    // TODO: texture indexing
    let texture = i_texture(vertex.i_other);
    var color: vec4<f32>;
    switch(texture) {
        case 0: {
            color = vec4(1.0, 1.0, 1.0, 1.0);
        }
        case default: {
            color = vec4(0.8, 0.8, 0.5, 1.0);
        }
    }
    
    var out: VertexOutput;
    out.clip_position = clip_position;
    out.color = color;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

fn i_width(other: u32) -> u32 {
    return (other >> WIDTH_SHIFT) & MASK6;
}

fn i_height(other: u32) -> u32 {
    return (other >> HEIGHT_SHIFT) & MASK6;
}

fn i_size(other: u32) -> vec2<u32> {
    return vec2<u32>(i_width(other), i_height(other));
}

fn i_face(other: u32) -> u32 {
    return (other >> FACE_SHIFT) & MASK3;
}

fn i_texture(other: u32) -> u32 {
    return other >> TEXTURE_SHIFT;
}

// TODO: drop the matrix and just do the math raw
fn custom_model_matrix(i_position: vec3<i32>, i_size: vec2<u32>, i_face: u32) -> mat4x4<f32> {
    let p = vec3<f32>(i_position);
    let s = vec2<f32>(i_size);
    let h = s / 2.0;

    switch(i_face) {
        case POS_X: {
            return mat4x4<f32>(
                vec4<f32>(0.0, 0.0, -s.x, 0.0),
                vec4<f32>(0.0, s.y, 0.0, 0.0),
                vec4<f32>(1.0, 0.0, 0.0, 0.0),
                vec4<f32>(p.x + 1.0, p.y + h.y, p.z + h.x, 1.0)
            );
        }
        case NEG_X: {
            return mat4x4<f32>(
                vec4<f32>(0.0, 0.0, s.x, 0.0),
                vec4<f32>(0.0, s.y, 0.0, 0.0),
                vec4<f32>(-1.0, 0.0, 0.0, 0.0),
                vec4<f32>(p.x, p.y + h.y, p.z + h.x, 1.0)
            );
        }
        case POS_Y: {
            return mat4x4<f32>(
                vec4<f32>(s.x, 0.0, 0.0, 0.0),
                vec4<f32>(0.0, 0.0, -s.y, 0.0),
                vec4<f32>(0.0, 1.0, 0.0, 0.0),
                vec4<f32>(p.x + h.x, p.y + 1.0, p.z + h.y, 1.0)
            );
        }
        case NEG_Y: {
            return mat4x4<f32>(
                vec4<f32>(-s.x, 0.0, 0.0, 0.0),
                vec4<f32>(0.0, 0.0, s.y, 0.0),
                vec4<f32>(0.0, -1.0, 0.0, 0.0),
                vec4<f32>(p.x + h.x, p.y, p.z + h.y, 1.0)
            );
        }
        case POS_Z: {
            return mat4x4<f32>(
                vec4<f32>(s.x, 0.0, 0.0, 0.0),
                vec4<f32>(0.0, s.y, 0.0, 0.0),
                vec4<f32>(0.0, 0.0, -1.0, 0.0),
                vec4<f32>(p.x + h.x, p.y + h.y, p.z + 1.0, 1.0)
            );
        }
        case default: { // NEG_Z
            return mat4x4<f32>(
                vec4<f32>(-s.x, 0.0, 0.0, 0.0),
                vec4<f32>(0.0, s.y, 0.0, 0.0),
                vec4<f32>(0.0, 0.0, 1.0, 0.0),
                vec4<f32>(p.x + h.x, p.y + h.y, p.z, 1.0)
            );
        }
    }
}
