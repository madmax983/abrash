#![cfg(feature = "nova")]
use abrash::ray::Ray;
use abrash::geometry::AABB;
use abrash::math::Vec3;

#[test]
fn test_ray_triangle_intersection() {
    let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));

    let v0 = Vec3::new(-1.0, -1.0, 0.0);
    let v1 = Vec3::new(1.0, -1.0, 0.0);
    let v2 = Vec3::new(0.0, 1.0, 0.0);

    // Expect intersection at t=5.0 (distance from 5.0 to 0.0)
    let hit_t = ray.intersect_triangle(v0, v1, v2);

    assert!(hit_t.is_some(), "Should hit triangle");
    let t = hit_t.unwrap();
    let point = ray.at(t);
    assert!((t - 5.0).abs() < 1e-3, "t should be 5.0, got {t}");
    assert!(
        (point.z - 0.0).abs() < 1e-4,
        "Point z should be 0.0, got {}",
        point.z
    );
}

#[test]
fn test_ray_aabb_intersection() {
    let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));

    let min = Vec3::new(-1.0, -1.0, -1.0);
    let max = Vec3::new(1.0, 1.0, 1.0);
    let aabb = AABB::new(min, max);

    // Expect intersection
    assert!(ray.intersect_aabb(&aabb).is_some(), "Should hit AABB");

    // Ray pointing away
    let ray_away = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 1.0, 0.0));
    assert!(
        ray_away.intersect_aabb(&aabb).is_none(),
        "Should miss AABB"
    );
}
