use abrash_core::geometry::*;
use abrash_core::math::*;

#[test]
fn havoc_test_cull_aabbs_avx2() {
    let mut aabbs = Vec::new();
    for _ in 0..8 {
        aabbs.push(AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)));
    }

    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    let proj = Mat4::perspective(std::f32::consts::PI / 4.0, 1.0, 0.1, 100.0);
    let frustum = abrash_core::culling::Frustum::from_matrix(proj * view);

    let mut results_avx2 = vec![false; 8];
    frustum.cull_aabbs_prealloc(&aabbs, &mut results_avx2);

    let mut results_scalar = vec![false; 8];
    for (i, aabb) in aabbs.iter().enumerate() {
        results_scalar[i] = frustum.intersects_aabb(aabb);
    }

    assert_eq!(results_avx2, results_scalar, "AVX2 AABB culling result does not match scalar path!");
}
