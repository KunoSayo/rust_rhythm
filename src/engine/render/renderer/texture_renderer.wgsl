


@group(0) @binding(0)
var s_diffuse: sampler;


struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs(input: VertexIn) -> VertexOut {
    var out: VertexOut;

    out.tex_coords = input.tex_coords;
    out.pos = vec4<f32>(input.position, 1.0, 1.0);

    return out;
}


struct FragUni {
    tint: vec4<f32>
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var<uniform> frag_uni: FragUni;

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {

    let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let result = object_color * frag_uni.tint;

    return result;
}
