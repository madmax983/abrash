use abrash::clipping::{ClippedTriangles, clip_triangle_to_frustum};
use abrash::math::Vec3;
use proptest::prelude::*;

// Scalar implementation of Sutherland-Hodgman clipping
// Copied from src/clipping.rs and stripped of SIMD optimizations
fn clip_triangle_scalar<V: Copy + std::fmt::Debug>(
    v0: V,
    v1: V,
    v2: V,
    get_pos: impl Fn(&V) -> (Vec3, f32),
) -> Vec<V> {
    // Return Vec<V> for easier comparison
    // We use a Vec instead of fixed array for the oracle to be safe
    let mut current_polygon = vec![v0, v1, v2];

    // 6 Planes
    // 1. Left: x >= -w -> x + w >= 0
    clip_plane(&mut current_polygon, |p, w| p.x + w, &get_pos);
    // 2. Right: x <= w -> w - x >= 0
    clip_plane(&mut current_polygon, |p, w| w - p.x, &get_pos);
    // 3. Bottom: y >= -w -> y + w >= 0
    clip_plane(&mut current_polygon, |p, w| p.y + w, &get_pos);
    // 4. Top: y <= w -> w - y >= 0
    clip_plane(&mut current_polygon, |p, w| w - p.y, &get_pos);
    // 5. Near: z >= -w -> z + w >= 0
    clip_plane(&mut current_polygon, |p, w| p.z + w, &get_pos);
    // 6. Far: z <= w -> w - z >= 0
    clip_plane(&mut current_polygon, |p, w| w - p.z, &get_pos);

    // Triangulate (Fan)
    let mut triangles = Vec::new();
    if current_polygon.len() >= 3 {
        let pivot = current_polygon[0];
        for i in 1..current_polygon.len() - 1 {
            triangles.push(pivot);
            triangles.push(current_polygon[i]);
            triangles.push(current_polygon[i + 1]);
        }
    }
    triangles
}

fn clip_plane<V: Copy>(
    polygon: &mut Vec<V>,
    dist_fn: impl Fn(Vec3, f32) -> f32,
    get_pos: &impl Fn(&V) -> (Vec3, f32),
) {
    if polygon.is_empty() {
        return;
    }

    let mut new_polygon = Vec::with_capacity(polygon.len() + 1);
    let mut prev_v = *polygon.last().unwrap();
    let (prev_pos, prev_w) = get_pos(&prev_v);
    let mut prev_d = dist_fn(prev_pos, prev_w);

    for &curr_v in polygon.iter() {
        let (curr_pos, curr_w) = get_pos(&curr_v);
        let curr_d = dist_fn(curr_pos, curr_w);

        if curr_d >= 0.0 {
            // Current is inside
            if prev_d < 0.0 {
                // Entered
                let t = prev_d / (prev_d - curr_d);
                new_polygon.push(prev_v.lerp(curr_v, t));
            }
            new_polygon.push(curr_v);
        } else {
            // Current is outside
            if prev_d >= 0.0 {
                // Exited
                let t = prev_d / (prev_d - curr_d);
                new_polygon.push(prev_v.lerp(curr_v, t));
            }
        }
        prev_v = curr_v;
        prev_d = curr_d;
    }

    *polygon = new_polygon;
}

// Convert ClippedTriangles to Vec<V> for comparison
fn clipped_to_vec<V: Copy>(clipped: &ClippedTriangles<V>) -> Vec<V> {
    let mut v = Vec::new();
    for i in 0..clipped.count * 3 {
        v.push(clipped[i]);
    }
    v
}

// Comparison with epsilon
fn compare_vertices(v1: &[(Vec3, f32)], v2: &[(Vec3, f32)]) {
    assert_eq!(
        v1.len(),
        v2.len(),
        "Vertex count mismatch: {} vs {}",
        v1.len(),
        v2.len()
    );
    for (i, (a, b)) in v1.iter().zip(v2.iter()).enumerate() {
        let (pa, wa) = a;
        let (pb, wb) = b;
        let diff = *pa - *pb;
        let eps = 0.001; // Allow some slack for float differences (especially SIMD vs Scalar order)

        assert!(
            diff.x.abs() < eps && diff.y.abs() < eps && diff.z.abs() < eps,
            "Vertex {i} pos mismatch: {pa:?} vs {pb:?}"
        );
        assert!((wa - wb).abs() < eps, "Vertex {i} w mismatch: {wa} vs {wb}");
    }
}

proptest! {
    #[test]
    fn fuzz_clipping(
        vx0 in -100.0f32..100.0, vy0 in -100.0f32..100.0, vz0 in -100.0f32..100.0, w0 in 0.1f32..100.0,
        vx1 in -100.0f32..100.0, vy1 in -100.0f32..100.0, vz1 in -100.0f32..100.0, w1 in 0.1f32..100.0,
        vx2 in -100.0f32..100.0, vy2 in -100.0f32..100.0, vz2 in -100.0f32..100.0, w2 in 0.1f32..100.0,
    ) {
        let v0 = (Vec3::new(vx0, vy0, vz0), w0);
        let v1 = (Vec3::new(vx1, vy1, vz1), w1);
        let v2 = (Vec3::new(vx2, vy2, vz2), w2);

        let get_pos = |v: &(Vec3, f32)| *v;

        // Run Optimized (SIMD)
        let result_simd = clip_triangle_to_frustum(v0, v1, v2, get_pos);
        let vec_simd = clipped_to_vec(&result_simd);

        // Run Scalar Oracle
        let vec_scalar = clip_triangle_scalar(v0, v1, v2, get_pos);

        compare_vertices(&vec_simd, &vec_scalar);
    }
}
