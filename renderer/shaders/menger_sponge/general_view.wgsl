//#define MARCH_MAX_STEPS 1000
//#define MAX_DISTANCE 10000.0

//#define STEPS_WHITE 0.
//#define STEPS_BLACK 1000.

//#define CAMERA_POSITION vec3(0., 0., -3.)
//#define CAMERA_ROTATION vec3(0., 3.14 / 3., 0.)
//#define CAMERA_FOCAL_LENGTH 1.4

//#define LIGHT_DIRECTION normalize(vec3(-3., 1., -3.))

//#define ENABLE_SHADOWS true
//#define SHADOWS_MAX_STEPS 1000

//#include "../main.wgsl"
//#include "./include.wgsl"

fn world_de(pos: vec3<f32>) -> DeResult {
    var npos = pos;

    var f: DeResult = new_de_result();

    var m = new_surface_material();

    f = de_min(f,
        menger_sponge_de(npos, 1., 20)
    );

    // f = de_min(f, de_with_material(
    //     sphere_de(npos + vec3(0., 0., -1.4), 0.2),
    //     m
    // ));

    return f;
}