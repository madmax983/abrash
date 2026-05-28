use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::TileRenderer;
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::zbuffer::ZBuffer;
use std::sync::Arc;

#[test]
fn test_scene_render() {
    // 1. Setup Renderer and Buffers
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    // 2. Setup Camera
    let eye = Vec3::new(0.0, 0.0, 5.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);

    let view = Mat4::look_at(eye, target, up);
    let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);
    let camera = Camera::new(view, proj);

    // 3. Create Scene
    let mut scene = Scene::new(camera);

    // 4. Add Objects
    // A single triangle centered at origin
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.5, 0.0));
    mesh.vertices.push(Vec3::new(-0.5, -0.5, 0.0));
    mesh.vertices.push(Vec3::new(0.5, -0.5, 0.0));
    mesh.indices.push([0, 1, 2]);
    let mesh = Arc::new(mesh);

    let transform = Mat4::identity();
    let object = SceneObject::new(mesh, transform, 0xFFFF_0000); // Red
    scene.add_object(object);

    // 5. Render
    let draw_list = scene.extract();
    renderer.begin_frame();
    for batch in &draw_list.batches {
        renderer.submit_mesh(
            &batch.indices,
            &draw_list.vertices[batch.vertex_range.start..batch.vertex_range.end],
            batch.color,
        );
    }
    renderer.end_frame(&mut fb, &mut zb);

    // 6. Verify
    // Check center pixel
    let center = fb.get_pixel(50, 50);
    assert_eq!(center, Some(0xFFFF_0000), "Center pixel should be red");
}
