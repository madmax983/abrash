#![cfg(feature = "nova")]
use abrash::experimental::raytracer::Ray;
use abrash::geometry::AABB;
use abrash::math::Vec3;

#[test]
fn test_ray_triangle_intersection() {
    let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));

    let v0 = Vec3::new(-1.0, -1.0, 0.0);
    let v1 = Vec3::new(1.0, -1.0, 0.0);
    let v2 = Vec3::new(0.0, 1.0, 0.0);

    // Expect intersection at t=5.0 (distance from 5.0 to 0.0)
    let hit = ray.intersect_triangle(v0, v1, v2, 0.0, 100.0);

    assert!(hit.is_some(), "Should hit triangle");
    let hit = hit.unwrap();
    assert!((hit.t - 5.0).abs() < 2e-3, "t should be 5.0, got {}", hit.t);
    assert!(
        (hit.point.z - 0.0).abs() < 2e-3,
        "Point z should be 0.0, got {}",
        hit.point.z
    );
}

#[test]
fn test_ray_aabb_intersection() {
    let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));

    let min = Vec3::new(-1.0, -1.0, -1.0);
    let max = Vec3::new(1.0, 1.0, 1.0);
    let aabb = AABB::new(min, max);

    // Expect intersection
    assert!(ray.intersect_aabb(&aabb, 0.0, 100.0), "Should hit AABB");

    // Ray pointing away
    let ray_away = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 1.0, 0.0));
    assert!(
        !ray_away.intersect_aabb(&aabb, 0.0, 100.0),
        "Should miss AABB"
    );
}

#[test]
fn test_ray_triangle_edge_cases() {
    let v0 = Vec3::new(-1.0, -1.0, 0.0);
    let v1 = Vec3::new(1.0, -1.0, 0.0);
    let v2 = Vec3::new(0.0, 1.0, 0.0);

    // 1. Ray parallel to the triangle (in the XY plane)
    let ray_parallel = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
    assert!(
        ray_parallel.intersect_triangle(v0, v1, v2, 0.0, 100.0).is_none(),
        "Parallel ray should miss"
    );

    // 2. Hit occurs before t_min (hit is at t=5, t_min=6)
    let ray_hit = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    assert!(
        ray_hit.intersect_triangle(v0, v1, v2, 6.0, 100.0).is_none(),
        "Hit before t_min should miss"
    );

    // 3. Hit occurs after t_max (hit is at t=5, t_max=4)
    assert!(
        ray_hit.intersect_triangle(v0, v1, v2, 0.0, 4.0).is_none(),
        "Hit after t_max should miss"
    );

    // 4. Ray misses the triangle bounds (hits the plane, but outside the triangle)
    let ray_miss_bounds = Ray::new(Vec3::new(2.0, 2.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
    assert!(
        ray_miss_bounds.intersect_triangle(v0, v1, v2, 0.0, 100.0).is_none(),
        "Ray outside triangle bounds should miss"
    );
}

#[test]
fn test_ray_aabb_edge_cases() {
    let min = Vec3::new(-1.0, -1.0, -1.0);
    let max = Vec3::new(1.0, 1.0, 1.0);
    let aabb = AABB::new(min, max);

    // 1. Ray originating inside the AABB
    let ray_inside = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
    // It should hit at t=1.0 (from 0 to 1), but wait - intersect_aabb returns a bool if it hits within [t_min, t_max]
    // Since origin is inside, tmin_xyz will be negative (-1) and tmax_xyz will be positive (1).
    // The condition is tmax >= tmin && tmax > t_min && tmin < t_max_bound.
    assert!(
        ray_inside.intersect_aabb(&aabb, 0.0, 100.0),
        "Ray inside AABB should hit"
    );

    // 2. Hit filtered out by t_min (hit is from -1 to 1, we require > 2)
    assert!(
        !ray_inside.intersect_aabb(&aabb, 2.0, 100.0),
        "Hit before t_min should miss"
    );

    // 3. Hit filtered out by t_max (hit is from -1 to 1, we require < -2)
    assert!(
        !ray_inside.intersect_aabb(&aabb, -10.0, -2.0),
        "Hit after t_max should miss"
    );

    // 4. Miss on a specific axis (ray starting outside and pointing away)
    let ray_miss_axis = Ray::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
    assert!(
        !ray_miss_axis.intersect_aabb(&aabb, 0.0, 100.0),
        "Ray pointing away from AABB should miss"
    );
}
