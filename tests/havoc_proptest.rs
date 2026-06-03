use proptest::prelude::*;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_core::texture::Texture;
use abrash_render::rasterizer::tile::TileRenderer;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    // 👺 Havoc: Test extremely large memory allocations to expose missing u32::MAX guards
    #[test]
    #[ignore = "👺 Havoc: Guaranteed OOM abort"]
    fn test_framebuffer_allocation_oom(width in (i32::MAX as u32)..(u32::MAX), height in 1..2u32) {
        let _ = Framebuffer::new(width, height);
    }

    #[test]
    #[ignore = "👺 Havoc: Guaranteed OOM abort"]
    fn test_zbuffer_allocation_oom(width in (i32::MAX as u32)..(u32::MAX), height in 1..2u32) {
        let _ = ZBuffer::new(width, height);
    }

    #[test]
    #[ignore = "👺 Havoc: Guaranteed OOM abort"]
    fn test_texture_allocation_oom(width in (i32::MAX as u32)..(u32::MAX), height in 1..2u32) {
        let _ = Texture::new(width, height);
    }

    #[test]
    #[ignore = "👺 Havoc: Guaranteed OOM abort"]
    fn test_tile_renderer_allocation_oom(width in (i32::MAX as u32)..(u32::MAX), height in 1..2u32) {
        let _ = TileRenderer::new(width, height);
    }
}
