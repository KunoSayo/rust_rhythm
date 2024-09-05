struct VertexOutput {
    @location(0) coord: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@location(0) a_pos: vec2<f32>, @builtin(vertex_index) idx: u32) -> VertexOutput {
    var out: VertexOutput;

    if ((idx & 1u) == 0u) {
        out.coord[0] = -1.0;
    } else {
        out.coord[0] = 1.0;
    }
    if ((idx & 3u) < 2u) {
        out.coord[1] = -1.0;
    } else {
        out.coord[1] = 1.0;
    }
    out.position = vec4<f32>(a_pos[0], a_pos[1], 0.5, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.coord[0] * in.coord[0] + in.coord[1] * in.coord[1] > 1.0) {
        discard;
    }
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
