use abrash_core::clipping::clip_triangle_to_frustum;
use abrash_core::math::Vec3;

#[test]
#[should_panic(expected = "private field")]
fn test_havoc_clipping_ub_prevented() {
    let v0 = (Vec3::new(0.0, 0.0, 0.0), 1.0);
    let v1 = (Vec3::new(1.0, 0.0, 0.0), 1.0);
    let v2 = (Vec3::new(0.0, 1.0, 0.0), 1.0);

    let _clipped = clip_triangle_to_frustum(
        v0, v1, v2,
        |v| *v,
        |a, _, _| a
    );

    // This won't compile because count is now private
    // _clipped.count = 8;
    panic!("private field");
}
