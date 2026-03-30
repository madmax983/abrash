use abrash::framebuffer::Framebuffer;
use abrash::math::Mat4;
use abrash::mesh::Mesh;
use abrash::rasterizer::tile::TileRenderer;
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::zbuffer::ZBuffer;
use proptest::prelude::*;
use std::sync::Arc;

proptest! {
    #[test]
    fn test_scene_render_no_panic(
        w in 16u32..200u32,
        h in 16u32..200u32,
        obj_count in 0usize..5usize,
    ) {
        if let Ok(mut fb) = Framebuffer::new(w, h)
            && let Ok(mut zb) = ZBuffer::new(w, h) {
                let mut renderer = TileRenderer::new(w, h);
                let camera = Camera::new(Mat4::identity(), Mat4::identity());
                let mut scene = Scene::new(camera);
                let mesh = Arc::new(Mesh::cube(1.0));
                for _ in 0..obj_count {
                    scene.add_object(SceneObject::new(mesh.clone(), Mat4::identity(), 0xFFFF_FFFF));
                }
                {
            let draw_list = scene.extract();
            renderer.begin_frame();
            for batch in &draw_list.batches {
                renderer.submit_mesh(&batch.indices, &batch.vertices, batch.color);
            }
            renderer.end_frame(&mut fb, &mut zb);
        }
            }
    }
}
