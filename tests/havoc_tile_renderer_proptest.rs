use proptest::prelude::*;
use abrash::rasterizer::tile::TileRenderer;
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::math::Vec3;

proptest! {
    #[test]
    fn test_tile_renderer_mesh_no_panic(
        w in 16u32..200u32,
        h in 16u32..200u32,
        v0x in any::<f32>(), v0y in any::<f32>(), v0z in any::<f32>(), v0w in any::<f32>(),
        v1x in any::<f32>(), v1y in any::<f32>(), v1z in any::<f32>(), v1w in any::<f32>(),
        v2x in any::<f32>(), v2y in any::<f32>(), v2z in any::<f32>(), v2w in any::<f32>(),
        color in any::<u32>(),
    ) {
        if let Ok(mut fb) = Framebuffer::new(w, h) {
            if let Ok(mut zb) = ZBuffer::new(w, h) {
                let mut renderer = TileRenderer::new(w, h);
                let verts = vec![
                    (Vec3::new(v0x, v0y, v0z), v0w),
                    (Vec3::new(v1x, v1y, v1z), v1w),
                    (Vec3::new(v2x, v2y, v2z), v2w),
                ];
                let indices = vec![[0, 1, 2]];
                renderer.submit_mesh(&indices, &verts, color);
                renderer.end_frame(&mut fb, &mut zb);
            }
        }
    }
}
