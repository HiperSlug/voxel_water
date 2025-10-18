#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}
#import bevy_pbr::forward_io::VertexOutput

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

    @location(8) instance_position: vec3<i32>,
    @location(9) instance_other: u32,
};

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

    // TODO: texture_indexing
    switch(instance_texture) {
        case 0: {
            out.color = vec4(0.5, 0.8, 0.8, 1.0);
        }
        case default: {
            out.color = vec4(0.8, 0.8, 0.5, 1.0);
        }
    }

    // TODO: lighting
    switch(instance_face) {
        case POS_X: {
            out.color *= 0.9;
        }
        case NEG_X: {
            out.color *= 0.1;
        }
        case POS_Y: {
            out.color *= 0.8;
        }
        case NEG_Y: {
            out.color *= 0.2;
        }
        case POS_Z: {
            out.color *= 0.7;
        }
        case default: { // && NEG_Z
            out.color *= 0.3;
        }
    }

    return out;
}

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
