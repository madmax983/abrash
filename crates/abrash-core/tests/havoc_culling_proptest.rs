use abrash_core::geometry::*;
use abrash_core::math::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn havoc_cull_aabbs_avx2_vs_scalar(
        min_x in -100.0f32..100.0, min_y in -100.0f32..100.0, min_z in -100.0f32..100.0,
        dx in 0.1f32..50.0, dy in 0.1f32..50.0, dz in 0.1f32..50.0,
        view_x in -10.0f32..10.0, view_y in -10.0f32..10.0, view_z in -10.0f32..10.0,
    ) {
        let aabb = AABB::new(Vec3::new(min_x, min_y, min_z), Vec3::new(min_x + dx, min_y + dy, min_z + dz));
        let aabbs = vec![aabb; 8];

        let view = Mat4::look_at(Vec3::new(view_x, view_y, view_z), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
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
}
