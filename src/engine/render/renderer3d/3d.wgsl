struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

struct Light {
    color: vec3<f32>,
    width: f32,
    dir: vec3<f32>,
    height: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var<uniform> light: Light;

struct PlaneVertexIn {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

struct PlaneVertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
}

@vertex
fn plane_vs(input: PlaneVertexIn) -> PlaneVertexOut {
    var out: PlaneVertexOut;

    out.tex_coords = input.tex_coords;
    out.pos = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.normal = input.normal;

    return out;
}

@vertex
fn plane_vs_full_tex(input: PlaneVertexIn, @builtin(vertex_index) vidx: u32) -> PlaneVertexOut {
    var out: PlaneVertexOut;

    if ((vidx & 1u) == 0u) {
        out.tex_coords.r = 0.0;
    } else {
        out.tex_coords.r = 1.0;
    }
    if (vidx < 2u) {
        out.tex_coords.g = 1.0;
    } else {
        out.tex_coords.g = 0.0;
    }
    out.pos = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.normal = input.normal;
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;

@fragment
fn plane_fs(in: PlaneVertexOut) -> @location(0) vec4<f32> {

    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let ambient_color = vec3<f32>(1.0, 1.0, 1.0) * 0.25;
    let diffuse_strength = max(dot(in.normal, light.dir), 0.0) * 0.75;
    let diffuse_color = light.color * diffuse_strength;
    let result = vec4<f32>((ambient_color + diffuse_color) * object_color.rgb, object_color.a);

    return result;
}

@fragment
fn plane_pos_tex_fs(in: PlaneVertexOut) -> @location(0) vec4<f32> {
    var pos = in.pos;

    var object_color: vec4<f32> = textureLoad(t_diffuse, vec2<u32>(u32(pos.x), u32(pos.y)), 0);

//    var surround = vec4<f32>(0.0, 0.0, 0.0, 0.0);
//
//    for (var i = -1.5; i <= 1.5; i += 1.0) {
//        for (var j = -1.5; j <= 1.5; j += 1.0) {
//            surround += textureSample(t_diffuse, s_diffuse, vec2<f32>((pos.x + i) / light.width, (pos.y + j) / light.height));
//        }
//    }
//    object_color += (1.0 - object_color.a) * surround / surround.a;


//    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);


    return object_color;
}
