use abrash_render::scene::{Scene, Camera};
use abrash_core::math::{Mat4, Vec3};

#[test]
fn test_scene_capacity() {
    let view = Mat4::look_at(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 0.0));
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);

    let scene1 = Scene::new(Camera::new(view, proj));
    assert_eq!(scene1.objects.capacity(), 0);

    let scene2 = Scene::with_capacity(Camera::new(view, proj), 100);
    assert_eq!(scene2.objects.capacity(), 100);
}
