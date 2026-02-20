use abrash::clipping::clip_triangle_to_frustum;
use abrash::math::Vec3;

// Helper to provide (Vec3, f32) as V
type Vertex = (Vec3, f32); // Pos, w

fn get_pos(v: &Vertex) -> (Vec3, f32) {
    *v
}

#[test]
fn test_clipping_behind_camera() {
    // Points "behind" the camera have w < 0 (if using -z convention).
    // Frustum checks: -w <= x <= w.
    // If w = -10. Range is [10, -10]. Empty.
    // So any point should be rejected.

    let v0 = (Vec3::new(0.0, 0.0, 0.0), -10.0);
    let v1 = (Vec3::new(1.0, 0.0, 0.0), -10.0);
    let v2 = (Vec3::new(0.0, 1.0, 0.0), -10.0);

    let result = clip_triangle_to_frustum(v0, v1, v2, get_pos);

    assert_eq!(result.count, 0, "Triangle with negative w should be fully culled");
}

#[test]
fn test_clipping_nan_safety() {
    // Sentry: Ensure NaN doesn't cause infinite loops or panics, and ideally is culled.
    let nan = f32::NAN;
    let v0 = (Vec3::new(nan, 0.0, 0.0), 1.0);
    let v1 = (Vec3::new(0.0, nan, 0.0), 1.0);
    let v2 = (Vec3::new(0.0, 0.0, nan), 1.0);

    // This should not panic and should return 0 triangles (safety guard).
    let result = clip_triangle_to_frustum(v0, v1, v2, get_pos);

    assert_eq!(result.count, 0, "Triangle with NaN should be culled for safety");
}

#[test]
fn test_clipping_infinity_safety() {
    // Similar to NaN, Infinite values should be culled.
    let inf = f32::INFINITY;
    let v0 = (Vec3::new(inf, 0.0, 0.0), 1.0);
    let v1 = (Vec3::new(0.0, inf, 0.0), 1.0);
    let v2 = (Vec3::new(0.0, 0.0, inf), 1.0);

    let result = clip_triangle_to_frustum(v0, v1, v2, get_pos);

    assert_eq!(result.count, 0, "Triangle with Infinity should be culled for safety");
}

#[test]
fn test_clipping_degenerate() {
    // All vertices identical.
    let v0 = (Vec3::new(0.0, 0.0, 0.0), 1.0);
    let v1 = (Vec3::new(0.0, 0.0, 0.0), 1.0);
    let v2 = (Vec3::new(0.0, 0.0, 0.0), 1.0);

    let result = clip_triangle_to_frustum(v0, v1, v2, get_pos);

    // Should be accepted (1 triangle, point) or rejected if logic handles degenerate.
    // The current implementation likely returns 1 triangle since it passes all checks.
    // If it returns 1, that's fine (rasterizer handles zero area).
    // If it returns 0, also fine.
    // Just ensure no panic.
    assert!(result.count <= 1);
}

#[test]
fn test_clipping_max_complexity() {
    // Construct a large triangle that covers the frustum [-1, 1]^3
    // but is clipped by it.
    // Triangle enclosing the square [-1, 1]x[-1, 1] at z=0.
    // Vertices: (0, 4, 0), (4, -2, 0), (-4, -2, 0). (w=1)

    // Top clip (y=1): clips top vertex.
    // Bottom clip (y=-1): clips bottom edge.
    // Left clip (x=-1): clips left corner.
    // Right clip (x=1): clips right corner.

    // Expected result: A polygon (likely hexagon or similar) triangulated.

    let w = 1.0;
    // Large triangle
    let v0 = (Vec3::new(0.0, 4.0, 0.0), w);
    let v1 = (Vec3::new(4.0, -3.0, 0.0), w);
    let v2 = (Vec3::new(-4.0, -3.0, 0.0), w);

    let result = clip_triangle_to_frustum(v0, v1, v2, get_pos);

    // Should produce multiple triangles
    // Clipping a triangle against a box (frustum) can produce up to 7-9 vertices.
    // A hexagon is 6 vertices -> 4 triangles.
    // A heptagon is 7 vertices -> 5 triangles.
    assert!(result.count > 1, "Should be clipped into multiple triangles, got {}", result.count);
    // Ensure we don't exceed buffer limits (implicit, would panic or corruption if bug exists)
}

#[test]
fn test_clipping_mixed_nan() {
    // v0 is NaN, v1 and v2 are valid and inside.
    // The Trivial Reject logic handles "all outside" (which NaN satisfies for all planes).
    // But if some are inside, it proceeds to clipping.
    // NaN propagates through interpolation, creating NaN vertices.
    // We want to ensure these are culled or handled safely.

    let nan = f32::NAN;
    let v0 = (Vec3::new(nan, 0.0, 0.0), 1.0);
    let v1 = (Vec3::new(0.0, 0.0, 0.0), 1.0); // Inside
    let v2 = (Vec3::new(0.0, 1.0, 0.0), 1.0); // Inside

    let result = clip_triangle_to_frustum(v0, v1, v2, get_pos);

    // If logic is robust, it should return 0 (cull) or at least valid non-NaN vertices.
    // But easiest robust behavior for a renderer is "Atomic Reject": if any vertex is trash, dump the triangle.
    assert_eq!(result.count, 0, "Triangle with mixed NaN/Valid should be fully culled");
}
