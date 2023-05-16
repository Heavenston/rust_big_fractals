@group(0)
@binding(0)
var otex: texture_storage_2d<rgba8unorm, write>;

const SSAA: u32 = 4u;

const MARCH_MAX_STEPS: i32 = 100;
const MAX_DISTANCE: f32 = 1000.0;
const HIT_DISTANCE: f32 = 0.001;

const STEPS_WHITE: f32 = 0.;
const STEPS_BLACK: f32 = 110.;

fn box(p: vec3<f32>, b: vec3<f32>) -> f32 {
    var q = abs(p) - b;
    return length(max(max(q.x,max(q.y,q.z)),0.)) + min(max(q.x,max(q.y,q.z)),0.);
}

fn distance_from_the_sphere(t: f32, pos: vec3<f32>) -> f32 {
    var radius: f32 = 1.;
    
    return length(pos - vec3(0., 0., 0.)) - radius;
}

fn world_de(t: f32, pos: vec3<f32>) -> f32 {
    var rot: f32 = 0.;
    var c: f32 = cos(rot);
    var s: f32 = sin(rot);
    var npos: vec3<f32> = vec3(
        pos.x * c - pos.z * s,
        pos.y,
        pos.z * c + pos.x * s
    );

    return distance_from_the_sphere(t, npos);
}

fn get_normal(t: f32, pos: vec3<f32>) -> vec3<f32> {
    var small_step_x = vec3(0.001, 0., 0.);
    var small_step_y = vec3(0., 0.001, 0.);
    var small_step_z = vec3(0., 0., 0.001);

    var normal = vec3(
        world_de(t, (pos + small_step_x)) - world_de(t, (pos - small_step_x)),
        world_de(t, (pos + small_step_y)) - world_de(t, (pos - small_step_y)),
        world_de(t, (pos + small_step_z)) - world_de(t, (pos - small_step_z))
    );

    return normalize(normal);
}

fn render_fragment(t: f32, origin: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    var traveled_distance: f32 = 0.;

    for (var i: i32 = 0; i < MARCH_MAX_STEPS; i++) {
        var current_pos: vec3<f32> = origin + (dir * traveled_distance);
        var ds: f32 = world_de(t, current_pos);

        if (ds < HIT_DISTANCE) {
            // return get_normal(t, current_pos);
            return vec3(1.) - vec3(1.) * ((clamp(f32(i), STEPS_WHITE, STEPS_BLACK) - STEPS_WHITE) / (STEPS_BLACK - STEPS_WHITE));
        }

        if (traveled_distance > MAX_DISTANCE) {
            break;
        }

        traveled_distance += ds;
    }

    return vec3(1., 0., 0.);
}

@compute
@workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var color_sum: vec3<f32> = vec3(0.);
    for (var dx: u32 = 0u; dx < SSAA; dx += 1u) {
        for (var dy: u32 = 0u; dy < SSAA; dy += 1u) {
            var uv: vec2<f32> = (vec2(
                f32(global_id.x * SSAA + dx) / f32(2048u * SSAA),
                f32(global_id.y * SSAA + dy) / f32(2048u * SSAA)
            ) * 2.) - vec2(1.);
            //uv /= 20.;
            var color: vec3<f32> = render_fragment(0., vec3(0., 0., -2.5), vec3(uv, 1.));
            color_sum += color;
        }
    }

    textureStore(otex, global_id.xy, vec4(color_sum / f32(SSAA * SSAA), 1.));
}