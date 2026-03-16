use abrash::culling::Frustum;
use abrash::math::{Mat4, Vec3};
use std::panic::catch_unwind;

#[test]
fn havoc_frustum_aabbs_prealloc_panic() {
    let result = catch_unwind(|| {
        let culler = Frustum::from_matrix(Mat4::identity());
        let aabb = abrash::geometry::AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let mut empty_results: Vec<bool> = vec![];
        // Havoc: Passing an empty results buffer while having 1 AABB.
        // It will panic inside cull_aabbs_prealloc.
        culler.cull_aabbs_prealloc(&[aabb], &mut empty_results);
    });

    assert!(
        result.is_err(),
        "Havoc: cull_aabbs_prealloc did not panic on empty results vector!"
    );
}
