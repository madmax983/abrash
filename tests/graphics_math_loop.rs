use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use abrash::geometry::{AABB, BoundingSphere};
use abrash::math::{Mat3, Vec3, Vec4, fast_atan2, lerp, remap, smoothstep};
use abrash::plane::{Frustum, Plane};
use abrash::quat::Quat;
use abrash::ray::Ray;
use abrash::transform::Transform;

const EPSILON: f32 = 1e-4;

fn assert_f32_close(actual: f32, expected: f32, label: &str) {
    assert!(
        (actual - expected).abs() < EPSILON,
        "{label}: actual={actual} expected={expected}"
    );
}

fn assert_vec3_close(actual: Vec3, expected: Vec3) {
    assert!(
        (actual.x - expected.x).abs() < EPSILON,
        "x mismatch: actual={} expected={}",
        actual.x,
        expected.x
    );
    assert!(
        (actual.y - expected.y).abs() < EPSILON,
        "y mismatch: actual={} expected={}",
        actual.y,
        expected.y
    );
    assert!(
        (actual.z - expected.z).abs() < EPSILON,
        "z mismatch: actual={} expected={}",
        actual.z,
        expected.z
    );
}

#[test]
fn quat_from_mat4_roundtrips_rotation() {
    let quat = Quat::from_euler(0.3, -0.7, 0.2).normalize();
    let roundtrip = Quat::from_mat4(quat.to_mat4());

    let original = quat.rotate_vec3(Vec3::new(0.3, -0.4, 0.5));
    let reconstructed = roundtrip.rotate_vec3(Vec3::new(0.3, -0.4, 0.5));
    assert_vec3_close(reconstructed, original);
}

#[test]
fn quat_inverse_undoes_rotation() {
    let quat = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
    let rotated = quat.rotate_vec3(Vec3::new(0.25, 0.0, 1.0));
    let restored = quat.inverse().rotate_vec3(rotated);
    assert_vec3_close(restored, Vec3::new(0.25, 0.0, 1.0));
}

#[test]
fn quat_nlerp_uses_shortest_arc() {
    let start = Quat::identity();
    let end = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
    let negated = Quat::new(-end.x, -end.y, -end.z, -end.w);

    let lhs = start.nlerp(&end, 0.5).rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
    let rhs = start
        .nlerp(&negated, 0.5)
        .rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
    assert_vec3_close(lhs, rhs);
}

#[test]
fn transform_transform_point_matches_matrix_path() {
    let transform = Transform::new(
        Vec3::new(3.0, -2.0, 5.0),
        Quat::from_euler(0.4, -0.2, 0.1),
        Vec3::new(2.0, 3.0, 0.5),
    );
    let point = Vec3::new(-1.5, 0.25, 2.0);

    let expected = transform.to_mat4().transform_point(point).0;
    let actual = transform.transform_point(point);
    assert_vec3_close(actual, expected);
}

#[test]
fn transform_inverse_transform_point_roundtrips_points() {
    let transform = Transform::new(
        Vec3::new(-4.0, 1.5, 2.0),
        Quat::from_euler(FRAC_PI_4, -0.3, 0.2),
        Vec3::new(1.5, 0.75, 2.0),
    );
    let point = Vec3::new(0.5, -2.0, 1.25);

    let world = transform.transform_point(point);
    let restored = transform.inverse_transform_point(world);
    assert_vec3_close(restored, point);
}

#[test]
fn transform_lerp_blends_all_components() {
    let start = Transform::identity();
    let end = Transform::new(
        Vec3::new(10.0, 5.0, -3.0),
        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2),
        Vec3::new(3.0, 5.0, 7.0),
    );

    let mid = start.lerp(end, 0.5);
    assert_vec3_close(mid.position, Vec3::new(5.0, 2.5, -1.5));
    assert_vec3_close(mid.scale, Vec3::new(2.0, 3.0, 4.0));

    let rotated = mid.rotation.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
    assert!((rotated.x - FRAC_PI_4.cos()).abs() < EPSILON);
    assert!((rotated.z + FRAC_PI_4.sin()).abs() < EPSILON);
}

#[test]
fn aabb_from_center_extents_roundtrips() {
    let aabb = AABB::from_center_extents(Vec3::new(3.0, -2.0, 1.0), Vec3::new(4.0, 5.0, 6.0));
    assert_vec3_close(aabb.center(), Vec3::new(3.0, -2.0, 1.0));
    assert_vec3_close(aabb.extents(), Vec3::new(4.0, 5.0, 6.0));
}

#[test]
fn aabb_queries_cover_common_overlap_cases() {
    let outer = AABB::new(Vec3::new(-2.0, -2.0, -2.0), Vec3::new(2.0, 2.0, 2.0));
    let inner = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
    let overlapping = AABB::new(Vec3::new(1.5, -0.5, -0.5), Vec3::new(3.0, 0.5, 0.5));
    let separate = AABB::new(Vec3::new(3.1, 0.0, 0.0), Vec3::new(4.0, 1.0, 1.0));

    assert!(outer.contains_point(Vec3::new(0.25, 0.25, 0.25)));
    assert!(!outer.contains_point(Vec3::new(3.0, 0.0, 0.0)));
    assert!(outer.contains_aabb(&inner));
    assert!(outer.intersects(&overlapping));
    assert!(!outer.intersects(&separate));
}

#[test]
fn aabb_union_and_include_point_expand_bounds() {
    let a = AABB::new(Vec3::new(-1.0, -2.0, 0.0), Vec3::new(2.0, 1.0, 3.0));
    let b = AABB::new(Vec3::new(-4.0, 0.5, -2.0), Vec3::new(1.0, 5.0, 1.0));

    let union = a.union(&b);
    assert_vec3_close(union.min, Vec3::new(-4.0, -2.0, -2.0));
    assert_vec3_close(union.max, Vec3::new(2.0, 5.0, 3.0));

    let expanded = union.include_point(Vec3::new(8.0, -3.0, 4.0));
    assert_vec3_close(expanded.min, Vec3::new(-4.0, -3.0, -2.0));
    assert_vec3_close(expanded.max, Vec3::new(8.0, 5.0, 4.0));
}

#[test]
fn aabb_transform_matches_corner_sweep() {
    let aabb = AABB::new(Vec3::new(-2.0, -1.0, -3.0), Vec3::new(1.0, 4.0, 2.0));
    let transform = Transform::new(
        Vec3::new(5.0, -3.0, 2.0),
        Quat::from_euler(0.2, 0.7, -0.4),
        Vec3::new(1.5, 0.5, 2.0),
    );

    let corners = [
        Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z),
        Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z),
        Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z),
        Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z),
        Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z),
        Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z),
        Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z),
        Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z),
    ];

    let transformed = aabb.transform(&transform.to_mat4());
    let expected = AABB::from_points(
        &corners
            .into_iter()
            .map(|corner| transform.transform_point(corner))
            .collect::<Vec<_>>(),
    );

    assert_vec3_close(transformed.min, expected.min);
    assert_vec3_close(transformed.max, expected.max);
}

// ── Scalar utilities ──────────────────────────────────────────────────────────

#[test]
fn lerp_interpolates_correctly() {
    assert_f32_close(lerp(0.0, 100.0, 0.0), 0.0, "lerp t=0");
    assert_f32_close(lerp(0.0, 100.0, 1.0), 100.0, "lerp t=1");
    assert_f32_close(lerp(0.0, 100.0, 0.25), 25.0, "lerp t=0.25");
}

#[test]
fn smoothstep_is_smooth_and_bounded() {
    assert_f32_close(smoothstep(0.0, 1.0, 0.0), 0.0, "ss edge0");
    assert_f32_close(smoothstep(0.0, 1.0, 1.0), 1.0, "ss edge1");
    // Derivative at edges should be 0 (smoothstep property)
    let h = 0.001;
    let d0 = (smoothstep(0.0, 1.0, h) - smoothstep(0.0, 1.0, 0.0)) / h;
    let d1 = (smoothstep(0.0, 1.0, 1.0) - smoothstep(0.0, 1.0, 1.0 - h)) / h;
    assert!(
        d0.abs() < 0.01,
        "smoothstep derivative at 0 should be ~0, got {d0}"
    );
    assert!(
        d1.abs() < 0.01,
        "smoothstep derivative at 1 should be ~0, got {d1}"
    );
}

#[test]
fn remap_maps_range_correctly() {
    assert_f32_close(remap(0.0, 0.0, 10.0, -1.0, 1.0), -1.0, "remap min");
    assert_f32_close(remap(10.0, 0.0, 10.0, -1.0, 1.0), 1.0, "remap max");
    assert_f32_close(remap(5.0, 0.0, 10.0, -1.0, 1.0), 0.0, "remap mid");
}

#[test]
fn fast_atan2_matches_std_within_tolerance() {
    for deg in (0..360).step_by(5) {
        let rad = (deg as f32) * PI / 180.0;
        let y = rad.sin();
        let x = rad.cos();
        let expected = y.atan2(x);
        let got = fast_atan2(y, x);
        assert!(
            (got - expected).abs() < 0.005,
            "fast_atan2 mismatch at {deg}°: got={got} expected={expected}"
        );
    }
}

// ── Mat3 ──────────────────────────────────────────────────────────────────────

#[test]
fn mat3_normal_transform_matches_mat4_for_rotation() {
    let angle = 1.2;
    let m4 = abrash::math::Mat4::rotation_y(angle);
    let m3 = Mat3::from_mat4(&m4);
    let it = m3.inverse_transpose();

    let normal = Vec3::new(0.3, -0.7, 0.6).normalize();
    let via_m4 = m4.transform_normal(normal);
    let via_m3_it = it.transform(normal).normalize();

    assert_vec3_close(via_m4, via_m3_it);
}

#[test]
fn mat3_inverse_cancels_rotation() {
    let m = Mat3::rotation_y(0.8);
    let inv = m.inverse();
    let v = Vec3::new(1.0, -2.0, 3.0);
    let roundtrip = inv.transform(m.transform(v));
    assert_vec3_close(roundtrip, v);
}

// ── Vec4 ──────────────────────────────────────────────────────────────────────

#[test]
fn vec4_dot_and_length() {
    let v = Vec4::new(0.0, 3.0, 4.0, 0.0);
    assert_f32_close(v.length(), 5.0, "vec4 length");
    assert_f32_close(v.dot(Vec4::new(1.0, 0.0, 0.0, 0.0)), 0.0, "vec4 dot perp");
}

#[test]
fn vec4_normalize_produces_unit_vector() {
    let v = Vec4::new(1.0, 2.0, 3.0, 4.0).normalize();
    assert_f32_close(v.length(), 1.0, "vec4 normalize length");
}

#[test]
fn vec4_xyz_extracts_vec3() {
    let v = Vec4::new(1.0, 2.0, 3.0, 99.0);
    assert_vec3_close(v.xyz(), Vec3::new(1.0, 2.0, 3.0));
}

// ── Plane & Frustum ───────────────────────────────────────────────────────────

#[test]
fn plane_signed_distance_is_correct() {
    // Use tolerance 0.01 due to fast_inv_sqrt approximation in normalize
    let p = Plane::from_point_normal(Vec3::new(0.0, 5.0, 0.0), Vec3::Y);
    assert!((p.distance(Vec3::new(0.0, 7.0, 0.0)) - 2.0).abs() < 0.01);
    assert!((p.distance(Vec3::new(0.0, 3.0, 0.0)) + 2.0).abs() < 0.01);
}

#[test]
fn frustum_culls_objects_outside() {
    let view = abrash::math::Mat4::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::UP);
    let proj = abrash::math::Mat4::perspective(FRAC_PI_2, 1.0, 1.0, 50.0);
    let frustum = Frustum::from_mat4(view * proj);

    // Origin is in frustum
    assert!(frustum.contains_point(Vec3::ZERO));
    // Far behind camera is outside
    assert!(!frustum.contains_point(Vec3::new(0.0, 0.0, 200.0)));
    // Large sphere at origin is inside
    assert!(frustum.contains_sphere(Vec3::ZERO, 1.0));
    // Tiny AABB at origin is inside
    let inner = AABB::new(Vec3::splat(-0.1), Vec3::splat(0.1));
    assert!(frustum.contains_aabb(&inner));
    // AABB behind camera is outside
    let behind = AABB::new(Vec3::new(0.0, 0.0, 100.0), Vec3::new(1.0, 1.0, 101.0));
    assert!(!frustum.contains_aabb(&behind));
}

// ── Ray ───────────────────────────────────────────────────────────────────────
// fast_inv_sqrt introduces ~0.5% error in normalize(); use loose tolerance here.
const RAY_TOL: f32 = 0.1;

#[test]
fn ray_at_returns_correct_point() {
    let ray = Ray::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(1.0, 0.0, 0.0));
    let p = ray.at(5.0);
    assert!((p.x - 6.0).abs() < RAY_TOL, "x: {}", p.x);
    assert!((p.y - 2.0).abs() < RAY_TOL, "y: {}", p.y);
    assert!((p.z - 3.0).abs() < RAY_TOL, "z: {}", p.z);
}

#[test]
fn ray_aabb_intersection_roundtrips() {
    let aabb = AABB::new(Vec3::new(-2.0, -1.0, -3.0), Vec3::new(2.0, 1.0, 3.0));
    let ray = Ray::new(Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.0, 0.0, -1.0));
    let (t_enter, _) = ray.intersect_aabb(&aabb).expect("hit");
    let hit_point = ray.at(t_enter);
    // Hit point should be on the AABB surface (z = 3.0)
    assert!(
        (hit_point.z - 3.0).abs() < RAY_TOL,
        "ray-aabb z at enter: {}",
        hit_point.z
    );
}

#[test]
fn ray_sphere_hit_nearest_root() {
    let sphere = BoundingSphere {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: 2.0,
    };
    let ray = Ray::new(Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.0, 0.0, -1.0));
    let t = ray.intersect_sphere(&sphere).expect("hit");
    // Nearest entry: distance from origin to sphere surface along -Z = 10 - 2 = 8
    assert!((t - 8.0).abs() < RAY_TOL, "ray-sphere t: {t}");
}

#[test]
fn ray_triangle_hit_at_origin() {
    let v0 = Vec3::new(-2.0, -2.0, 0.0);
    let v1 = Vec3::new(2.0, -2.0, 0.0);
    let v2 = Vec3::new(0.0, 2.0, 0.0);
    let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    let t = ray.intersect_triangle(v0, v1, v2).expect("hit");
    assert!((t - 5.0).abs() < RAY_TOL, "ray-triangle t: {t}");
}

#[test]
fn transform_batch_path_matches_scalar_path() {
    let transform = Transform::new(
        Vec3::new(1.0, -2.0, 3.0),
        Quat::from_euler(0.4, -0.3, 0.2),
        Vec3::new(2.0, 0.5, 1.5),
    );
    let points = vec![
        Vec3::new(-2.0, 0.0, 1.0),
        Vec3::new(0.5, 1.5, -3.0),
        Vec3::new(4.0, -1.0, 2.0),
    ];

    let batch = transform.transform_points(&points);
    assert_eq!(batch.len(), points.len());
    for (actual, point) in batch.iter().zip(points.iter()) {
        let expected = transform.transform_point(*point);
        assert_vec3_close(*actual, expected);
    }
}

#[test]
fn transform_vector_batch_matches_scalar_path() {
    let transform = Transform::new(
        Vec3::new(1.0, -2.0, 3.0),
        Quat::from_euler(0.25, -0.5, 0.125),
        Vec3::new(0.5, 2.0, 1.5),
    );
    let vectors = vec![
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, -1.0),
        Vec3::new(-2.0, 0.5, 0.25),
    ];

    let batch = transform.transform_vectors(&vectors);
    assert_eq!(batch.len(), vectors.len());
    for (actual, vector) in batch.iter().zip(vectors.iter()) {
        let expected = transform.transform_vector(*vector);
        assert_vec3_close(*actual, expected);
    }
}

#[test]
fn inverse_transform_batch_roundtrips_points() {
    let transform = Transform::new(
        Vec3::new(-3.0, 4.5, 1.0),
        Quat::from_euler(0.4, 0.2, -0.6),
        Vec3::new(1.25, 0.75, 2.5),
    );
    let local_points = vec![
        Vec3::new(-1.0, 2.0, 0.5),
        Vec3::new(0.0, -3.0, 4.0),
        Vec3::new(2.5, 1.5, -2.0),
    ];
    let world_points = transform.transform_points(&local_points);

    let restored = transform.inverse_transform_points(&world_points);
    assert_eq!(restored.len(), local_points.len());
    for (actual, expected) in restored.iter().zip(local_points.iter()) {
        assert_vec3_close(*actual, *expected);
    }
}

#[test]
fn transform_in_place_paths_match_allocating_paths() {
    let transform = Transform::new(
        Vec3::new(2.0, -1.0, 4.0),
        Quat::from_euler(0.35, -0.45, 0.2),
        Vec3::new(1.5, 0.75, 2.25),
    );

    let points = vec![
        Vec3::new(-1.0, 0.5, 2.0),
        Vec3::new(3.0, -2.0, 1.5),
        Vec3::new(0.0, 4.0, -3.5),
    ];
    let vectors = vec![
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, -1.0),
        Vec3::new(-1.5, 0.25, 0.5),
    ];

    let expected_points = transform.transform_points(&points);
    let expected_vectors = transform.transform_vectors(&vectors);

    let mut in_place_points = points;
    transform.transform_points_in_place(&mut in_place_points);
    for (actual, expected) in in_place_points.iter().zip(expected_points.iter()) {
        assert_vec3_close(*actual, *expected);
    }

    let mut in_place_vectors = vectors;
    transform.transform_vectors_in_place(&mut in_place_vectors);
    for (actual, expected) in in_place_vectors.iter().zip(expected_vectors.iter()) {
        assert_vec3_close(*actual, *expected);
    }
}

#[test]
fn inverse_transform_in_place_roundtrips_world_points() {
    let transform = Transform::new(
        Vec3::new(-6.0, 2.0, 1.0),
        Quat::from_euler(-0.2, 0.6, -0.35),
        Vec3::new(1.25, 1.25, 1.25),
    );
    let local_points = vec![
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(-2.5, 0.0, 4.0),
        Vec3::new(0.25, -1.25, -0.5),
    ];
    let mut world_points = transform.transform_points(&local_points);

    transform.inverse_transform_points_in_place(&mut world_points);
    for (actual, expected) in world_points.iter().zip(local_points.iter()) {
        assert_vec3_close(*actual, *expected);
    }
}

#[test]
fn transform_then_matches_matrix_composition() {
    let a = Transform::new(
        Vec3::new(1.0, -2.0, 0.5),
        Quat::from_euler(0.2, -0.3, 0.1),
        Vec3::new(1.2, 1.2, 1.2),
    );
    let b = Transform::new(
        Vec3::new(-4.0, 0.5, 2.0),
        Quat::from_euler(-0.4, 0.25, 0.6),
        Vec3::new(0.75, 0.75, 0.75),
    );
    let combined = a.then(b);
    let point = Vec3::new(-1.0, 3.0, 2.25);

    let matrix_expected = (a.to_mat4() * b.to_mat4()).transform_point(point).0;
    let actual = combined.transform_point(point);
    assert_vec3_close(actual, matrix_expected);
}

#[test]
fn aabb_ray_and_sphere_queries_hit_expected_ranges() {
    let aabb = AABB::new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(2.0, 1.0, 4.0));

    let (t_near, t_far) = aabb
        .intersects_ray(Vec3::new(-5.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0))
        .expect("expected ray hit");
    assert!((t_near - 4.0).abs() < EPSILON);
    assert!((t_far - 7.0).abs() < EPSILON);

    assert!(aabb.intersects_sphere(Vec3::new(2.5, 0.0, 0.0), 0.5));
    assert!(!aabb.intersects_sphere(Vec3::new(3.0, 3.0, 3.0), 0.5));
}
