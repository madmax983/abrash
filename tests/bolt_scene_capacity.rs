use abrash_core::math::{Mat4, Vec3};
use abrash_render::scene::{Camera, Scene};

#[test]
fn test_scene_with_capacity_allocates_correctly() {
    let view = Mat4::look_at(Vec3::new(0.0, 50.0, 50.0), Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
    let proj = Mat4::perspective(1.0, 1.0, 0.1, 1000.0);
    let camera = Camera::new(view, proj);

    let scene = Scene::with_capacity(camera, 123);

    assert_eq!(scene.objects.capacity(), 123);
    assert!(scene.objects.is_empty());
}
