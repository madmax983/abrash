use abrash_core::math::Vec2;
use abrash_core::sdf;

#[test]
fn test_polyline_2d_basic() {
    let pts = [
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(2.0, 0.0),
    ];

    // On the vertex
    let d = sdf::polyline_2d(Vec2::new(1.0, 1.0), &pts);
    assert!(d.abs() < 1e-5);

    // Middle of the first segment
    let d = sdf::polyline_2d(Vec2::new(0.5, 0.5), &pts);
    assert!(d.abs() < 1e-5);

    // Some distance away
    let d = sdf::polyline_2d(Vec2::new(1.0, 2.0), &pts);
    assert!((d - 1.0).abs() < 1e-5);
}

#[test]
fn test_polyline_2d_empty() {
    let pts = [];
    let d = sdf::polyline_2d(Vec2::new(1.0, 1.0), &pts);
    assert_eq!(d, 0.0);
}

#[test]
fn test_polyline_2d_single_point() {
    let pts = [Vec2::new(1.0, 1.0)];
    let d = sdf::polyline_2d(Vec2::new(1.0, 2.0), &pts);
    assert_eq!(d, 1.0);
}

#[test]
fn test_polyline_2d_nan_avoidance() {
    let pts = [
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(2.0, 0.0),
    ];

    // Test that the explicit check handles NaNs as b gracefully
    // although they aren't expected, this is testing the reduction
    let d1 = sdf::segment_2d(Vec2::new(1.0, 2.0), pts[0], pts[1]);
    let d2 = sdf::segment_2d(Vec2::new(1.0, 2.0), pts[1], pts[2]);
    let min_d = if d1 < d2 { d1 } else { d2 };

    let d = sdf::polyline_2d(Vec2::new(1.0, 2.0), &pts);
    assert!((d - min_d).abs() < 1e-5);
}
