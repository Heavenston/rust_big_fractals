const SSAA: u32 = 1u;

const MARCH_MAX_STEPS: i32 = 2000;
const MAX_DISTANCE: f32 = 1000.0;
const HIT_DISTANCE: f32 = 0.00025;

const STEPS_WHITE: f32 = 0.;
const STEPS_BLACK: f32 = 2000.;

struct DeResult {
    distance: f32,
    color: vec3<f32>,
}

fn de_min(a: DeResult, b: DeResult) -> DeResult {
    if (a.distance > b.distance) {
        return b;
    }
    return a;
}
fn de_max(a: DeResult, b: DeResult) -> DeResult {
    if (a.distance > b.distance) {
        return a;
    }
    return b;
}

fn modulo(a: f32, b: f32) -> f32 {
    return ((a % b) + b) % b;
}

fn modulo_vec3(a: vec3<f32>, b: f32) -> vec3<f32> {
    return (((a + vec3(b / 2.)) % b) + vec3(b)) % b - vec3(b / 2.);
}

fn box_de(pos: vec3<f32>, box_size: vec3<f32>) -> DeResult {
    var result: DeResult;
    var q = abs(pos) - box_size;
    result.distance =
        length(max(max(q.x,max(q.y,q.z)),0.)) + min(max(q.x,max(q.y,q.z)),0.);
    result.color = vec3(1., 1., 1.);
    return result;
}

fn sphere_de(pos: vec3<f32>, radius: f32) -> DeResult {
    var result: DeResult;
    result.distance = length(pos - vec3(0., 0., 0.)) - radius;
    result.color = vec3(1., 1., 1.);
    return result;
}

fn menger_cross_de(point: vec3<f32>, size: f32, extent: f32) -> DeResult {
    return de_min(
        box_de(point, vec3(size, extent, size)), de_min(
        box_de(point, vec3(extent, size, size)),
        box_de(point, vec3(size, size, extent))
    ));
}

fn menger_sponge_de(point: vec3<f32>, side: f32, iterations: i32) -> DeResult {
    var distance: f32 = box_de(point, vec3(side)).distance;

    var factor = 1.;
    var cross_middle = point;

    for (var i = 0; i < 9; i += 1) {
        factor /= 3.;

        var mpoint: vec3<f32> = vec3(
            modulo(point.x + side * factor, side * factor * 6.) - side * factor,
            modulo(point.y + side * factor, side * factor * 6.) - side * factor,
            modulo(point.z + side * factor, side * factor * 6.) - side * factor,
        );
        var cross = menger_cross_de(mpoint, side * factor, 100.).distance;
        distance = max(
            distance,
            -cross
        );

        cross_middle += vec3(side, 0., 0.) * factor * 2.;
    }

    var result: DeResult;
    result.distance = distance;
    result.color = vec3(1., 1., 1.);
    return result;
}

fn world_de(pos: vec3<f32>) -> DeResult {
    var npos = pos;

    // return menger_sponge_de(npos, 1., 2);
    // return menger_cross(npos, 0.3, 1.);
    return sphere_de(modulo_vec3(pos, 5.), 1.);
    // return distance_from_the_sphere(npos);
}

fn get_normal(pos: vec3<f32>) -> vec3<f32> {
    var small_step_x = vec3(0.001, 0., 0.);
    var small_step_y = vec3(0., 0.001, 0.);
    var small_step_z = vec3(0., 0., 0.001);

    return normalize(vec3(
        world_de(pos + small_step_x).distance - world_de(pos - small_step_x).distance,
        world_de(pos + small_step_y).distance - world_de(pos - small_step_y).distance,
        world_de(pos + small_step_z).distance - world_de(pos - small_step_z).distance,
    ));
}

fn cast_ray(origin: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    var traveled_distance: f32 = 0.;

    for (var i: i32 = 0; i < MARCH_MAX_STEPS; i++) {
        var current_pos: vec3<f32> = origin + (dir * traveled_distance);
        var rs = world_de(current_pos);

        if (rs.distance < HIT_DISTANCE) {
            var tint = vec3(1.) - vec3(1.) * ((clamp(f32(i), STEPS_WHITE, STEPS_BLACK) - STEPS_WHITE) / (STEPS_BLACK - STEPS_WHITE));
            tint *= rs.color;
            tint *= abs(get_normal(current_pos));
            return tint;
        }

        if (traveled_distance > MAX_DISTANCE) {
            break;
        }

        traveled_distance += rs.distance / 2.;
    }

    return vec3(0., 0., 0.);
}

struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var result: VertexOutput;
    result.tex_coord = vec2(0.);
    result.pos = vec4(0.);
    if (in_vertex_index == 0u) {
        result.tex_coord = vec2(0., 0.);
        result.pos = vec4(-1., -1., 0., 1.);
    }
    else if (in_vertex_index == 1u) {
        result.tex_coord = vec2(1., 0.);
        result.pos = vec4(1., -1., 0., 1.);
    }
    else if (in_vertex_index == 2u) {
        result.tex_coord = vec2(0., 1.);
        result.pos = vec4(-1., 1., 0., 1.);
    }
    else if (in_vertex_index == 3u) {
        result.tex_coord = vec2(1., 1.);
        result.pos = vec4(1., 1., 0., 1.);
    }
    else if (in_vertex_index == 4u) {
        result.tex_coord = vec2(1., 0.);
        result.pos = vec4(1., -1., 0., 1.);
    }
    else if (in_vertex_index == 5u) {
        result.tex_coord = vec2(0., 1.);
        result.pos = vec4(-1., 1., 0., 1.);
    }

    return result;
}

@group(0)
@binding(0)
var<uniform> uv_transform: mat3x3<f32>;

@fragment
fn fragment_main(v: VertexOutput) -> @location(0) vec4<f32> {
    var color_sum: vec3<f32> = vec3(0.);
    var uv: vec2<f32> = v.tex_coord * 2. - vec2(1.);
    uv = (vec3(uv, 1.) * uv_transform).xy;
    uv /= 1.3;
    return vec4(cast_ray(vec3(0., 0., -3.), vec3(uv, 1.)), 1.);
}

// @compute
// @workgroup_size(1)
// fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
//     var color_sum: vec3<f32> = vec3(0.);
//     for (var dx: u32 = 0u; dx < SSAA; dx += 1u) {
//         for (var dy: u32 = 0u; dy < SSAA; dy += 1u) {
//             var uv: vec2<f32> = (vec2(
//                 f32(global_id.x * SSAA + dx) / f32(2048u * SSAA),
//                 f32(global_id.y * SSAA + dy) / f32(2048u * SSAA)
//             ) * 2.) - vec2(1.);
//             var color: vec3<f32> = render_fragment(vec3(0., 0., -3.), vec3(uv, 1.));
//             color_sum += color;
//         }
//     }

//     textureStore(otex, global_id.xy, vec4(color_sum / f32(SSAA * SSAA), 1.));
// }