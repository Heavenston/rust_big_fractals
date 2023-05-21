const SSAA: u32 = 1u;

const MARCH_MAX_STEPS: i32 = 10000;
const MAX_DISTANCE: f32 = 100000000.0;
const HIT_DISTANCE: f32 = 0.00005;

const STEPS_WHITE: f32 = 0.;
const STEPS_BLACK: f32 = 500.;

struct SurfaceMaterial {
    color: vec3<f32>,
    reflexion_strength: f32,
    diffuse_strength: f32,
}

struct DeResult {
    distance: f32,
    material: SurfaceMaterial,
}

fn new_surface_material() -> SurfaceMaterial {
    var d: SurfaceMaterial;
    d.color = vec3(1.);
    d.reflexion_strength = 0.;
    d.diffuse_strength = 1.;
    return d;
}

fn new_de_result() -> DeResult {
    var d: DeResult;
    d.distance = MAX_DISTANCE;
    d.material = new_surface_material();
    return d;
}

fn de_inv(a: DeResult) -> DeResult {
    var na = a;
    na.distance *= -1.;
    return na;
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

fn de_with_material(a: DeResult, mat: SurfaceMaterial) -> DeResult {
    var na = a;
    na.material = mat;
    return na;
}

fn modulo(a: f32, b: f32) -> f32 {
    return ((a % b) + b) % b;
}

fn modulo_vec3(a: vec3<f32>, b: f32) -> vec3<f32> {
    return (((a + vec3(b / 2.)) % b) + vec3(b)) % b - vec3(b / 2.);
}

fn box_de(pos: vec3<f32>, box_size: vec3<f32>) -> DeResult {
    var result: DeResult = new_de_result();
    var q = abs(pos) - box_size;
    result.distance =
        length(max(max(q.x,max(q.y,q.z)),0.)) + min(max(q.x,max(q.y,q.z)),0.);
    return result;
}

fn sphere_de(pos: vec3<f32>, radius: f32) -> DeResult {
    var result: DeResult = new_de_result();
    result.distance = length(pos - vec3(0., 0., 0.)) - radius;
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

    for (var i = 0; i < 6; i += 1) {
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

    var result: DeResult = new_de_result();
    result.distance = distance;
    return result;
}

const MANDELBULB_ITERATIONS: i32 = 50; // Increase to increase the fractal precision
const MANDELBULB_POWER: f32 = 8.;

fn mandelbulb_de(pos: vec3<f32>) -> DeResult {
    let Bailout: f32 = 2.;
    let Power: f32 = MANDELBULB_POWER;

    var z: vec3<f32> = pos;
    var dr: f32 = 1.0;
    var r: f32 = 0.0;

    var material: SurfaceMaterial = new_surface_material();
    for (var i: i32 = 0; i < MANDELBULB_ITERATIONS; i++) {
        r = length(z);

        if (r > Bailout) {
            let x = clamp(f32(i) / 23., 0., 1.);
            material.color =
                (vec3(1., 0., 0.) * x)        +
                (vec3(1., 1., 1.) * (1. - x));
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

    var result = new_de_result();
    result.material = material;
    result.distance = 0.5 * log(r) * r / dr;
    return result;
}

fn world_de(pos: vec3<f32>) -> DeResult {
    var npos = pos;

    var f: DeResult = new_de_result();

    // f = de_min(f, de_max(
    //     box_de(npos - vec3(1., 0., 0.), vec3(1.)),
    //     sphere_de(npos - vec3(1., 0., 0.), 1.4)
    // ));
    // f = de_min(f, de_color(
    //     box_de(npos + vec3(1., -0.4, 0.), vec3(0.2)),
    //     vec3(1., 0., 0.)
    // ));
    // f = de_min(f, de_color(
    //     box_de(npos + vec3(1., 0.4, 0.), vec3(0.2)),
    //     vec3(0., 1., 0.)
    // ));

    // f = de_min(f,
    //     sphere_de(modulo_vec3(pos, 5.), 1.)
    // );

    // f = de_min(f,
    //     menger_sponge_de(npos, 1., 2)
    // );

    f = de_min(f,
        mandelbulb_de(npos)
    );

    var m = new_surface_material();
    m.reflexion_strength = 0.9;
    m.diffuse_strength = 0.;

    f = de_min(f,
        de_with_material(sphere_de(npos + vec3(0., 0., -1.4), 0.2), m)
    );

    // return box_de(npos, vec3(1.));
    // return menger_cross_de(npos, 1., 1.);
    // return sphere_de(npos, 1.2);
    // return de_min(
    //     sphere_de(npos + vec3(0., 0.5, 0.), 1.),
    //     sphere_de(npos - vec3(0., 0.5, 0.), 1.),
    // );

    return f;
}

fn get_normal(pos: vec3<f32>) -> vec3<f32> {
    let small_step = 0.000001;
    let small_step_x = vec3(1., 0., 0.) * small_step;
    let small_step_y = vec3(0., 1., 0.) * small_step;
    let small_step_z = vec3(0., 0., 1.) * small_step;

    return normalize(vec3(
        world_de(pos + small_step_x).distance - world_de(pos - small_step_x).distance,
        world_de(pos + small_step_y).distance - world_de(pos - small_step_y).distance,
        world_de(pos + small_step_z).distance - world_de(pos - small_step_z).distance,
    ));
}

struct RayCastResult {
    hit: bool,
    steps: i32,
    point: vec3<f32>,
    normal: vec3<f32>,
    distance: f32,

    material: SurfaceMaterial,
}

fn cast_ray(origin: vec3<f32>, dir: vec3<f32>) -> RayCastResult {
    var traveled_distance: f32 = max(HIT_DISTANCE, world_de(origin).distance);

    for (var i: i32 = 0; i < MARCH_MAX_STEPS; i++) {
        var current_pos: vec3<f32> = origin + (dir * traveled_distance);
        var rs = world_de(current_pos);

        if (rs.distance < HIT_DISTANCE) {
            var result: RayCastResult;
            result.steps = i;
            result.hit = true;
            result.normal = get_normal(current_pos);
            result.distance = rs.distance;
            result.point = current_pos;

            result.material = rs.material;
            return result;
        }

        if (traveled_distance > MAX_DISTANCE) {
            break;
        }

        traveled_distance += max(HIT_DISTANCE, rs.distance);
    }

    var result: RayCastResult;
    result.hit = false;
    result.material.color = vec3(0.3, 0.3, 0.8);
    return result;
}

fn shaded_ray(origin: vec3<f32>, dir: vec3<f32>) -> RayCastResult {
    var rs = cast_ray(origin, dir);
    if (!rs.hit) { return rs; }

    var oo_tint = 1. - (clamp(f32(rs.steps), STEPS_WHITE, STEPS_BLACK) - STEPS_WHITE) / (STEPS_BLACK - STEPS_WHITE);
    rs.material.color *= oo_tint;

    // let LIGHT_POSITION = vec3(0., 3., 0.);
    let light_direction = normalize(vec3(0.2, 1., 1.));

    if (rs.material.diffuse_strength > 0.) {
        let hit_light = cast_ray(rs.point + light_direction * HIT_DISTANCE, light_direction);
        var diffuse_intensity: f32 = 0.2;
        if (!hit_light.hit) {
            diffuse_intensity = clamp(dot(rs.normal, light_direction), 0.2, 1.);
        }
        rs.material.color *= 1. * (1. - rs.material.diffuse_strength) + diffuse_intensity * rs.material.diffuse_strength;
    }

    return rs;
}

fn cast_bouncing_ray(init_point: vec3<f32>, init_dir: vec3<f32>) -> vec3<f32> {
    var rs = shaded_ray(init_point, init_dir);
    var total_color = rs.material.color;
    var dir = init_dir;

    for (
        var i = 0u;
        rs.hit && rs.material.reflexion_strength > 0. && i < 20u;
        i += 1u
    ) {
        var reflexion = dir - 2. * dot(dir, rs.normal) * rs.normal;

        var nrs = shaded_ray(rs.point + reflexion * HIT_DISTANCE, reflexion);
        var strength = rs.material.reflexion_strength;
        total_color = total_color * (1. - strength) + nrs.material.color * strength;

        dir = reflexion;
        rs = nrs;
    }

    return total_color;
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

    var angle = (3.14 / 4.) * 5.4;
    // angle = (3.14 / 4.) * 4.5;
    var rot_mat =  mat3x3(
        cos(angle),  0.,  sin(angle),
        0.,          1.,  0.,
        -sin(angle), 0.,  cos(angle),
    );

    var ray_direction = normalize(vec3(uv, 4.));
    ray_direction *= rot_mat;

    var cam_pos = vec3(1.0, 0., -3.);
    cam_pos *= rot_mat;

    return vec4(cast_bouncing_ray(cam_pos, ray_direction), 1.);
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