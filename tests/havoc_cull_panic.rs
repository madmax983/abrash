use abrash::culling::Frustum;
use abrash::geometry::{AABB, BoundingSphere};
use abrash::math::{Mat4, Vec3};
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_cull_panic(
        x in any::<f32>(),
        y in any::<f32>(),
        z in any::<f32>(),
        r in any::<f32>(),
    ) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let frustum = Frustum::from_matrix(Mat4::identity());
            let spheres = vec![BoundingSphere { center: Vec3::new(x, y, z), radius: r }];
            let _ = frustum.cull_spheres(&spheres);
            let mut results = vec![false; 1];
            frustum.cull_spheres_prealloc(&spheres, &mut results);
            let aabbs = vec![AABB { min: Vec3::new(x, y, z), pad0: 0.0, max: Vec3::new(x+r, y+r, z+r), pad1: 0.0 }];
            frustum.cull_aabbs_prealloc(&aabbs, &mut results);
        }));
        assert!(result.is_ok());
    }
}
