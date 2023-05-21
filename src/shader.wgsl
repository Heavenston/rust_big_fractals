

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var result: VertexOutput;
    result.uv = vec2(0.);
    result.pos = vec4(0.);
    if (in_vertex_index == 0u) {
        result.uv = vec2(0., 0.);
        result.pos = vec4(-1., -1., 0., 1.);
    }
    else if (in_vertex_index == 1u) {
        result.uv = vec2(1., 0.);
        result.pos = vec4(1., -1., 0., 1.);
    }
    else if (in_vertex_index == 2u) {
        result.uv = vec2(0., 1.);
        result.pos = vec4(-1., 1., 0., 1.);
    }
    else if (in_vertex_index == 3u) {
        result.uv = vec2(1., 1.);
        result.pos = vec4(1., 1., 0., 1.);
    }
    else if (in_vertex_index == 4u) {
        result.uv = vec2(1., 0.);
        result.pos = vec4(1., -1., 0., 1.);
    }
    else if (in_vertex_index == 5u) {
        result.uv = vec2(0., 1.);
        result.pos = vec4(-1., 1., 0., 1.);
    }

    return result;
}

@fragment
fn fragment_main(v: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(v.uv, 0., 1.);
}
