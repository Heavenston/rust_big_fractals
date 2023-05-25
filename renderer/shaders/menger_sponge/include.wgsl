
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

    for (var i = 0; i < iterations; i += 1) {
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
    result.material = new_surface_material();
    return result;
}

