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
    fn test_scene_render_fuzz_params(
        w in 16u32..200u32,
        h in 16u32..200u32,
        obj_count in 0usize..5usize,
        tx in any::<f32>(), ty in any::<f32>(), tz in any::<f32>(),
        sx in any::<f32>(), sy in any::<f32>(), sz in any::<f32>(),
        rx in any::<f32>(), ry in any::<f32>(), rz in any::<f32>(),
    ) {
        if let Ok(mut fb) = Framebuffer::new(w, h)
            && let Ok(mut zb) = ZBuffer::new(w, h) {
                let mut renderer = TileRenderer::new(w, h);
                let camera = Camera::new(Mat4::identity(), Mat4::identity());
                let mut scene = Scene::new(camera);
                let mesh = Arc::new(Mesh::cube(1.0));

                let transform = Mat4::scale(sx, sy, sz)
                    * Mat4::rotation_x(rx)
                    * Mat4::rotation_y(ry)
                    * Mat4::rotation_z(rz)
                * Mat4::translation(tx, ty, tz);

                for _ in 0..obj_count {
                    scene.add_object(SceneObject::new(mesh.clone(), transform, 0xFFFF_FFFF));
                }
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
            }
    }
}
