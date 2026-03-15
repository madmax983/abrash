use proptest::prelude::*;
use abrash::rasterizer::tile::TileRenderer;
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::math::Vec3;

proptest! {
    #[test]
    fn test_tile_renderer_fuzz_indices(
        w in 16u32..200u32,
        h in 16u32..200u32,
        i0 in 3usize..10000usize, // Ensure out of bounds index (>= 3)
        i1 in any::<usize>(),
        i2 in any::<usize>(),
    ) {
        if let Ok(_fb) = Framebuffer::new(w, h) {
            if let Ok(_zb) = ZBuffer::new(w, h) {
                let mut renderer = TileRenderer::new(w, h);
                let verts = vec![
                    (Vec3::new(0.0, 0.0, 0.0), 1.0),
                    (Vec3::new(1.0, 0.0, 0.0), 1.0),
                    (Vec3::new(0.0, 1.0, 0.0), 1.0),
                ];
                let indices = vec![[i0, i1, i2]];

                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    renderer.submit_mesh(&indices, &verts, 0xFFFFFFFF);
                }));

                // Havoc: Assert that it ALWAYS panics due to out of bounds
                assert!(result.is_err(), "Havoc: Expected panic on out of bounds index, but it succeeded!");
            }
        }
    }
}
