
@group(0)
@binding(0)
var<uniform> viewport_transform: mat3x3<f32>;

@group(1)
@binding(0)
var splr: sampler;
@group(1)
@binding(1)
var txture: texture_2d<f32>;
@group(1)
@binding(2)
var<uniform> image_transform: mat3x3<f32>; 

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var pos = vec2(0., 0.);
    var uv = vec2(0., 0.);

    if (in_vertex_index == 0u) {
        uv = vec2(0., 0.);
        pos = vec2(-1., -1.);
    }
    else if (in_vertex_index == 1u) {
        uv = vec2(1., 0.);
        pos = vec2(1., -1.);
    }
    else if (in_vertex_index == 2u) {
        uv = vec2(0., 1.);
        pos = vec2(-1., 1.);
    }
    else if (in_vertex_index == 3u) {
        uv = vec2(1., 1.);
        pos = vec2(1., 1.);
    }
    else if (in_vertex_index == 4u) {
        uv = vec2(1., 0.);
        pos = vec2(1., -1.);
    }
    else if (in_vertex_index == 5u) {
        uv = vec2(0., 1.);
        pos = vec2(-1., 1.);
    }

    pos = (vec3(pos.xy, 1.) * viewport_transform * image_transform).xy;

    var result: VertexOutput;
    result.uv = uv;
    result.pos = vec4(pos, 0., 1.);

    return result;
}

@fragment
fn fragment_main(v: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(txture, splr, v.uv);
}
