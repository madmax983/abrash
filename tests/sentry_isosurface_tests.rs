use abrash::experimental::isosurface::*;
use abrash::math::Vec3;

#[test]
fn test_extract_isosurface_empty_field() {
    let sdf = |_p: Vec3| 100.0; // Everything is outside
    let min = Vec3::new(-1.0, -1.0, -1.0);
    let max = Vec3::new(1.0, 1.0, 1.0);

    let mesh = extract_isosurface(sdf, min, max, 2);

    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
}

#[test]
fn test_extract_isosurface_full_field() {
    let sdf = |_p: Vec3| -100.0; // Everything is inside
    let min = Vec3::new(-1.0, -1.0, -1.0);
    let max = Vec3::new(1.0, 1.0, 1.0);

    let mesh = extract_isosurface(sdf, min, max, 2);

    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
}

#[test]
fn test_extract_isosurface_plane() {
    let sdf = |p: Vec3| p.y; // Plane at y=0
    let min = Vec3::new(-1.0, -1.0, -1.0);
    let max = Vec3::new(1.0, 1.0, 1.0);

    let mesh = extract_isosurface(sdf, min, max, 4);

    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.indices.is_empty());

    for v in mesh.vertices {
        // Since it's a plane at y=0, all generated vertices should be at y=0
        assert!(v.y.abs() < 1e-4);
    }
}

#[test]
fn test_polygonize_tetrahedron_colinear_edge() {
    // Edge case: test with distance difference very close to 0
    // Edge case tested via extract_isosurface

    // Should not panic, panic could happen in `t = v_a / (v_a - v_b)` if division by zero occurs
    // We can't call private function polygonize_tetrahedron, but we can call extract_isosurface
    // and provide an SDF that produces a very small value difference.
    let sdf = |p: Vec3| {
        if p.x < 0.0 { -1e-6 } else { 1e-6 }
    };
    let min = Vec3::new(-1.0, -1.0, -1.0);
    let max = Vec3::new(1.0, 1.0, 1.0);

    let mesh = extract_isosurface(sdf, min, max, 2);
    assert!(!mesh.vertices.is_empty());
}
