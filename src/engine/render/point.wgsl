struct VertexOutput {
    @location(0) c: vec4<f32>,
    @location(1) coord: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@location(0) a_color: vec4<f32>, @location(1) a_pos: vec2<f32>, @builtin(vertex_index) idx: u32) -> VertexOutput {
    var out: VertexOutput;
    out.c = a_color;

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
    let dis = in.coord[0] * in.coord[0] + in.coord[1] * in.coord[1];
    if (dis > 1.0) {
        discard;
    }
//    let dis = 1.0 - pow(1.0 - dis, 3.0);

    return in.c;
}
