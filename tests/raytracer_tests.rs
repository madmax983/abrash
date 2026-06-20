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
        (hit.point.z - 0.0).abs() < 1e-4,
        "Point z should be 0.0, got {}",
        hit.point.z
    );
}

#[test]
fn test_ray_triangle_intersection_table() {
    let v0 = Vec3::new(-1.0, -1.0, 0.0);
    let v1 = Vec3::new(1.0, -1.0, 0.0);
    let v2 = Vec3::new(0.0, 1.0, 0.0);

    struct TestCase {
        name: &'static str,
        origin: Vec3,
        dir: Vec3,
        expected_hit: bool,
    }

    let cases = vec![
        TestCase {
            name: "hit center",
            origin: Vec3::new(0.0, 0.0, 5.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
            expected_hit: true,
        },
        TestCase {
            name: "miss - parallel to triangle",
            origin: Vec3::new(0.0, 0.0, 5.0),
            dir: Vec3::new(1.0, 0.0, 0.0),
            expected_hit: false,
        },
        TestCase {
            name: "miss - ray out of bounds (u/v)",
            origin: Vec3::new(2.0, 2.0, 5.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
            expected_hit: false,
        },
        TestCase {
            name: "miss - triangle behind ray",
            origin: Vec3::new(0.0, 0.0, -5.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
            expected_hit: false,
        },
    ];

    for case in cases {
        let ray = Ray::new(case.origin, case.dir);
        let hit = ray.intersect_triangle(v0, v1, v2, 0.0, 100.0);
        assert_eq!(
            hit.is_some(),
            case.expected_hit,
            "Case failed: {}",
            case.name
        );
    }
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
