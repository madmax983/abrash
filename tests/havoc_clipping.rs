use abrash::clipping::clip_triangle_against_near_plane;
use abrash::math::Vec4;

#[test]
fn havoc_clip_triangle_panic() {
    let lerp = |a: (Vec4, f32), b: (Vec4, f32), t: f32| (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t);

    let v1 = (Vec4::new(1.0, 1.0, 1.0, 0.05), 0.0);
    let v2 = (Vec4::new(1.0, 1.0, 1.0, 0.05), 0.0);
    let v3 = (Vec4::new(1.0, 1.0, 1.0, 1.0), 0.0);

    let get_w_panic = |_v: &(Vec4, f32)| {
        panic!("Havoc: get_w panic");
        1.0
    };
    clip_triangle_against_near_plane(v1, v2, v3, get_w_panic, lerp);
}
