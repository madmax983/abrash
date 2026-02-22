use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::tile_renderer::TileRenderer;
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use std::sync::Arc;

#[test]
fn test_scene_render_allocations_logic() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    let proj = Mat4::perspective(1.0, 1.0, 0.1, 100.0);
    let camera = Camera::new(view, proj);
    let mut scene = Scene::new(camera);

    let mesh = Arc::new(Mesh::cube(1.0));

    // Add multiple objects to trigger the loop multiple times
    for i in 0..10 {
        let transform = Mat4::translation(i as f32, 0.0, 0.0);
        scene.add_object(SceneObject::new(mesh.clone(), transform, 0xFFFFFFFF));
    }

    // This should run without panics or errors
    scene.render(&mut renderer, &mut fb, &mut zb);
}
