use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use abrash::geometry::AABB;
use abrash::math::Vec3;
use abrash::quat::Quat;
use abrash::transform::Transform;

const EPSILON: f32 = 1e-4;

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

    let mut in_place_points = points.clone();
    transform.transform_points_in_place(&mut in_place_points);
    for (actual, expected) in in_place_points.iter().zip(expected_points.iter()) {
        assert_vec3_close(*actual, *expected);
    }

    let mut in_place_vectors = vectors.clone();
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
