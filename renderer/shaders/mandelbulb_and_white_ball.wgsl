#define MARCH_MAX_STEPS 10000
#define MAX_DISTANCE 100000000.0
#define HIT_DISTANCE 0.000025

#define STEPS_WHITE 0.
#define STEPS_BLACK 350.

#define CAMERA_POSITION vec3(0., 0., -3.)
#define CAMERA_ROTATION vec3(0., (3.14 / 4.) * 11.5, 0.)

#include "main.wgsl"

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

const MANDELBULB_ITERATIONS: i32 = 200; // Increase to increase the fractal precision
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
            let x = clamp(
                f32(max(0, i + -4)) / 15.,
                0.,
                1.
            );
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

    var m = new_surface_material();

    f = de_min(f,
        mandelbulb_de(npos)
    );

    f = de_min(f, de_with_material(
        sphere_de(npos + vec3(0., 0., -1.4), 0.2),
        m
    ));

    return f;
}
