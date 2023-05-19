const SSAA: u32 = 1u;

const MARCH_MAX_STEPS: i32 = 10000000;
const MAX_DISTANCE: f32 = 1000.0;
const HIT_DISTANCE: f32 = 0.00005;

const STEPS_WHITE: f32 = 0.;
const STEPS_BLACK: f32 = 100.;

fn box(p: vec3<f32>, b: vec3<f32>) -> f32 {
    var q = abs(p) - b;
    return length(max(max(q.x,max(q.y,q.z)),0.)) + min(max(q.x,max(q.y,q.z)),0.);
}

fn distance_from_the_sphere(pos: vec3<f32>) -> f32 {
    var radius: f32 = 1.;
    
    return length(pos - vec3(0., 0., 0.)) - radius;
}

fn menger_cross(point: vec3<f32>, size: f32, extent: f32) -> f32 {
    return min(
        box(point, vec3(size, extent, size)), min(
        box(point, vec3(extent, size, size)),
        box(point, vec3(size, size, extent))
    ));
}

fn mengerSponge(point: vec3<f32>, side: f32, iterations: i32) -> f32 {
    return menger_cross(point, side / 9., 10.);

    // var distance: f32 = box(point, vec3(side));

    // var factor = 1.;

    // for (var i = 0; i < 2; i += 1) {
    //     factor /= 3.;

    //     var mpoint = (point + vec3(side * factor * 3.));
    //     var cross = menger_cross(mpoint, side * factor, 10.);
    //     distance = max(
    //         distance,
    //         -cross
    //     );
    // }
    
    // return distance;
}

fn world_de(pos: vec3<f32>) -> f32 {
    var rot: f32 = -45.;
    var c: f32 = cos(rot);
    var s: f32 = sin(rot);
    var npos: vec3<f32> = vec3(
        pos.x * c - pos.z * s,
        pos.y,
        pos.z * c + pos.x * s
    );

    // return mengerSponge(npos, 1., 2);
    // return menger_cross(npos, 0.3, 1.);
    return distance_from_the_sphere((abs(npos) + vec3(5.)) % 10. - vec3(5.));
    // return distance_from_the_sphere(npos);
}

fn get_normal(pos: vec3<f32>) -> vec3<f32> {
    var small_step_x = vec3(0.001, 0., 0.);
    var small_step_y = vec3(0., 0.001, 0.);
    var small_step_z = vec3(0., 0., 0.001);

    var normal = vec3(
        world_de((pos + small_step_x)) - world_de((pos - small_step_x)),
        world_de((pos + small_step_y)) - world_de((pos - small_step_y)),
        world_de((pos + small_step_z)) - world_de((pos - small_step_z))
    );

    return normalize(normal);
}

fn render_fragment(origin: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    var traveled_distance: f32 = 0.;

    for (var i: i32 = 0; i < MARCH_MAX_STEPS; i++) {
        var current_pos: vec3<f32> = origin + (dir * traveled_distance);
        var ds: f32 = world_de(current_pos);

        if (ds < HIT_DISTANCE) {
            // return get_normal(t, current_pos);
            return vec3(1.) - vec3(1.) * ((clamp(f32(i), STEPS_WHITE, STEPS_BLACK) - STEPS_WHITE) / (STEPS_BLACK - STEPS_WHITE));
        }

        if (traveled_distance > MAX_DISTANCE) {
            break;
        }

        traveled_distance += ds;
    }

    // return vec3(1., 0., 1.);
    return vec3(world_de(origin + dir) / 100.);
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

@fragment
fn fragment_main(v: VertexOutput) -> @location(0) vec4<f32> {
    var color_sum: vec3<f32> = vec3(0.);
    var uv: vec2<f32> = v.tex_coord * 2. - vec2(1.);
    return vec4(render_fragment(vec3(0., 0., -3.), vec3(uv, 1.)), 1.);
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