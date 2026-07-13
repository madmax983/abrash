use abrash_render::rasterizer::TileRenderer;
use proptest::prelude::*;

proptest! {
    #[test]
    #[ignore = "👹 Havoc: Integer Overflow in TileRenderer dimensions bypasses safety checks"]
    fn test_tile_renderer_validate_overflow(width in 1..=u32::MAX, height in 1..=u32::MAX) {
        let _ = TileRenderer::new(width, height);
    }
}
