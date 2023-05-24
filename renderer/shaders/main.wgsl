//#default MARCH_MAX_STEPS 100
//#default MAX_DISTANCE 10000.0

//#default STEPS_WHITE 0.
//#default STEPS_BLACK 100.

//#default CAMERA_POSITION vec3(0., 0., -3.)
//#default CAMERA_ROTATION vec3(0., 0., 0.)
//#default CAMERA_FOCAL_LENGTH 1.0

//#default ENABLE_SHADOWS false

@group(0)
@binding(0)
var<uniform> uv_transform: mat3x3<f32>;

@group(0)
@binding(1)
var<uniform> screen_size: vec2<u32>;

struct SurfaceMaterial {
    color: vec3<f32>,
    reflexion_strength: f32,
    diffuse_strength: f32,
}

struct DeResult {
    distance: f32,
    material: SurfaceMaterial,
}

struct RayCastConfig {
    origin: vec3<f32>,
    direction: vec3<f32>,
    start_distance: f32,
    max_distance: f32,

    hit_distance: f32,
    hit_scaling: f32,
}

struct RayCastResult {
    hit: bool,
    steps: i32,
    point: vec3<f32>,
    normal: vec3<f32>,
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


fn get_normal(pos: vec3<f32>, small_step: f32) -> vec3<f32> {
    let small_step_x = vec3(1., 0., 0.) * small_step;
    let small_step_y = vec3(0., 1., 0.) * small_step;
    let small_step_z = vec3(0., 0., 1.) * small_step;

    return normalize(vec3(
        world_de(pos + small_step_x).distance - world_de(pos - small_step_x).distance,
        world_de(pos + small_step_y).distance - world_de(pos - small_step_y).distance,
        world_de(pos + small_step_z).distance - world_de(pos - small_step_z).distance,
    ));
}

fn cast_ray(config: RayCastConfig) -> RayCastResult {
    var traveled_distance: f32 = config.start_distance;

    for (var i: i32 = 0; i < MARCH_MAX_STEPS; i++) {
        var current_pos: vec3<f32> =
            config.origin + (config.direction * traveled_distance);
        var hit_distance =
            config.hit_distance * config.hit_scaling * traveled_distance;

        var rs = world_de(current_pos);
        if (rs.distance < hit_distance) {
            var result: RayCastResult;
            result.steps = i;
            result.hit = true;
            result.normal = get_normal(current_pos, hit_distance);
            result.distance = rs.distance;
            result.point = current_pos;

            result.material = rs.material;
            return result;
        }

        if (traveled_distance > config.max_distance) {
            break;
        }

        traveled_distance += max(hit_distance, rs.distance);
    }

    var result: RayCastResult;
    result.hit = false;
    result.material.color = vec3(0.3, 0.3, 0.8);
    return result;
}

fn shaded_ray(config: RayCastConfig) -> RayCastResult {
    var rs = cast_ray(config);
    if (!rs.hit) { return rs; }

    let light_direction = normalize(vec3(0.2, 1., 1.));

    if (rs.material.diffuse_strength > 0.) {
        var should_light = true;

        // Shadows
        if (ENABLE_SHADOWS) {
            var light_hit_config = config;
            light_hit_config.origin =
                rs.point + rs.normal * 0.01;
            light_hit_config.start_distance = rs.distance;
            light_hit_config.direction = light_direction;
            let hit_light = cast_ray(light_hit_config);
            should_light = should_light && hit_light.hit;
        }

        var diffuse_intensity: f32 = 0.2;
        if (should_light) {
            diffuse_intensity =
                clamp(dot(rs.normal, light_direction), 0.2, 1.);
        }
        rs.material.color *= 1. * (1. - rs.material.diffuse_strength) +
                             diffuse_intensity * rs.material.diffuse_strength;
    }

    return rs;
}

fn cast_bouncing_ray(config: RayCastConfig) -> vec3<f32> {
    var rs = shaded_ray(config);
    var total_color = rs.material.color;
    var dir = config.direction;

    var oo_tint = 1. - (clamp(f32(rs.steps), STEPS_WHITE, STEPS_BLACK) - STEPS_WHITE) / (STEPS_BLACK - STEPS_WHITE);
    total_color *= oo_tint;

    for (
        var i = 0u;
        rs.hit && rs.material.reflexion_strength > 0. && i < 10u;
        i += 1u
    ) {
        var reflexion_config = config;
        reflexion_config.direction =
            dir - 2. * dot(dir, rs.normal) * rs.normal;
        reflexion_config.origin = 
            rs.point + reflexion_config.direction * config.hit_distance;
        reflexion_config.start_distance = rs.distance;

        var nrs = shaded_ray(reflexion_config);
        var strength = rs.material.reflexion_strength;
        total_color = total_color * (1. - strength) +
                      nrs.material.color * strength;

        dir = reflexion_config.direction;
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

@fragment
fn fragment_main(v: VertexOutput) -> @location(0) vec4<f32> {
    var color_sum: vec3<f32> = vec3(0.);
    var uv: vec2<f32> = v.tex_coord * 2. - vec2(1.);
    uv = (vec3(uv, 1.) * uv_transform).xy;

    var y_angle = CAMERA_ROTATION.y;
    var rot_mat =  mat3x3(
        cos(y_angle),  0.,  sin(y_angle),
        0.,            1.,  0.,
        -sin(y_angle), 0.,  cos(y_angle),
    );

    var ray_direction = normalize(vec3(uv, CAMERA_FOCAL_LENGTH));
    ray_direction *= rot_mat;

    var cam_pos = CAMERA_POSITION;
    cam_pos *= rot_mat;

    var config: RayCastConfig;
    config.origin = cam_pos;
    config.direction = ray_direction;
    config.start_distance = 0.;
    config.max_distance = 1000000.;
    var screen_max_size = max(screen_size.x, screen_size.y);
    config.hit_distance = 1. / f32(screen_max_size);
    config.hit_scaling = 1. / CAMERA_FOCAL_LENGTH;

    return vec4(cast_bouncing_ray(config), 1.);
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