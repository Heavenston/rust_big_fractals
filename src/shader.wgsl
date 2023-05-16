@group(0)
@binding(0)
var otex: texture_storage_2d<rgba8unorm, write>;

const MARCH_MAX_STEPS: i32 = 100;
const MAX_DISTANCE: f32 = 1000.0;
const HIT_DISTANCE: f32 = 0.001;

const MANDELBULB_ITERATIONS: i32 = 10; // Increase to increase the fractal precision
const MANDELBULB_POWER: f32 = 8.;

fn mandelbulb(t: f32, pos: vec3<f32>) -> f32 {
    let Bailout: f32 = 1.15;
    let Power: f32 = MANDELBULB_POWER;

    var z: vec3<f32> = pos;
    var dr: f32 = 1.0;
    var r: f32 = 0.0;
    for (var i: i32 = 0; i < MANDELBULB_ITERATIONS; i++) {
        r = length(z);

        if (r > Bailout) {
            break;
        }

        // convert to polar coordinates
        var theta: f32 = acos(z.z / r);
        var phi: f32 = atan2(z.y, z.x);
        dr = pow(r, Power - 1.) * Power * dr + 1.;

        // scale and rotate the point
        var zr: f32 = pow(r, Power);
        theta = theta*Power;
        phi = phi*Power;

        // convert back to cartesian coordinates
        z = vec3(
            sin(theta)*cos(phi),
            sin(phi)*sin(theta),
            cos(theta)
        ) * zr;
        z = z + pos;
    }
    return 0.5 * log(r) * r / dr;
}

fn distance_from_the_sphere(t: f32, pos: vec3<f32>) -> f32 {
    var radius: f32 = 1.;
    
    return length(pos - vec3(0., 0., 0.)) - radius;
}

fn world_de(t: f32, pos: vec3<f32>) -> f32 {
    return mandelbulb(t, pos);
}

fn render_fragment(t: f32, origin: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    var traveled_distance: f32 = 0.;

    for (var i: i32 = 0; i < MARCH_MAX_STEPS; i++) {
        var current_pos: vec3<f32> = origin + (dir * traveled_distance);
        var ds: f32 = world_de(t, current_pos);

        if (ds < HIT_DISTANCE) {
            return vec3(1., 1., 1.) * (1. - f32(i) / f32(MARCH_MAX_STEPS));
        }

        if (traveled_distance > MAX_DISTANCE) {
            break;
        }

        traveled_distance += ds;
    }

    return vec3(1., 0., 0.);
}

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var uv: vec2<f32> = (vec2(
        f32(global_id.x) / 500.,
        f32(global_id.y) / 500.
    ) * 2.) - vec2(1.);
    var color: vec3<f32> = render_fragment(0., vec3(0., 0., -2.), vec3(uv, 1.));

    textureStore(otex, global_id.xy, vec4(color, 1.));
}