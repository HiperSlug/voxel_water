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
const NEG_Z: u32 = 5;

const U16_MAX_F32: f32 = 65535.0;

struct Vertex {
    @location(0) origin: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    @location(3) i_pos: vec3<i32>,
    @location(4) i_other: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let size = i_size(vertex.i_other);
    let face = i_face(vertex.i_other);

    let matrix = face_model_matrix(
        vertex.i_pos,
        size,
        face,
    );

    let clip_position = matrix * vec4<f32>(vertex.origin, 1.0);

    // placeholderw
    let texture = i_texture(vertex.i_other);
    let c = f32(texture) / U16_MAX_F32; 
    let color = vec4<f32>(c, c, c, 1.0);

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
    (other >> WIDTH_SHIFT) & MASK6
}

fn i_height(other: u32) -> u32 {
    (other >> HEIGHT_SHIFT) & MASK6
}

fn i_size(other: u32) -> vec2<u32> {
    vec2<u32>(i_width(other), i_height(other))
}

fn i_face(other: u32) -> u32 {
    (other >> FACE_SHIFT) & MASK3
}

fn i_texture(other: u32) -> u32 {
    other >> TEXTURE_SHIFT
}

// AI
fn face_model_matrix(
    pos: vec3<i32>,
    size: vec2<u32>,
    face: u32,
) -> mat4x4<f32> {
    let p = vec3<f32>(pos);
    let s = vec2<f32>(size);
    
    switch(face) {
		case POS_X: return mat4x4<f32>(
			vec4<f32>(0.0, 0.0, -s.x, 0.0),
			vec4<f32>(0.0, s.y, 0.0, 0.0),
			vec4<f32>(1.0, 0.0, 0.0, 0.0),
			vec4<f32>(p.x + 0.5, p.y + s.y * 0.5, p.z + s.x * 0.5, 1.0)
		);
		case NEG_X: return mat4x4<f32>(
			vec4<f32>(0.0, 0.0, s.x, 0.0),
			vec4<f32>(0.0, s.y, 0.0, 0.0),
			vec4<f32>(-1.0, 0.0, 0.0, 0.0),
			vec4<f32>(p.x - 0.5, p.y + s.y * 0.5, p.z + s.x * 0.5, 1.0)
		);
		case POS_Y: return mat4x4<f32>(
			vec4<f32>(s.x, 0.0, 0.0, 0.0),
			vec4<f32>(0.0, 0.0, s.y, 0.0),
			vec4<f32>(0.0, 1.0, 0.0, 0.0),
			vec4<f32>(p.x + s.x * 0.5, p.y + 0.5, p.z + s.y * 0.5, 1.0)
		);
		case NEG_Y: return mat4x4<f32>(
			vec4<f32>(s.x, 0.0, 0.0, 0.0),
			vec4<f32>(0.0, 0.0, -s.y, 0.0),
			vec4<f32>(0.0, -1.0, 0.0, 0.0),
			vec4<f32>(p.x + s.x * 0.5, p.y - 0.5, p.z + s.y * 0.5, 1.0)
		);
		case POS_Z: return mat4x4<f32>(
			vec4<f32>(s.x, 0.0, 0.0, 0.0),
			vec4<f32>(0.0, s.y, 0.0, 0.0),
			vec4<f32>(0.0, 0.0, 1.0, 0.0),
			vec4<f32>(p.x + s.x * 0.5, p.y + s.y * 0.5, p.z + 0.5, 1.0)
		);
		case NEG_Z: return mat4x4<f32>(
			vec4<f32>(s.x, 0.0, 0.0, 0.0),
			vec4<f32>(0.0, s.y, 0.0, 0.0),
			vec4<f32>(0.0, 0.0, -1.0, 0.0),
			vec4<f32>(p.x + s.x * 0.5, p.y + s.y * 0.5, p.z - 0.5, 1.0)
		);
        case default: return mat4x4<f32>(1.0);
	}
}
