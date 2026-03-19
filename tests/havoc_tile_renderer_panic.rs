use abrash::math::Vec3;
use abrash::rasterizer::tile::TileRenderer;
use proptest::prelude::*;

proptest! {
    #[test]
    #[allow(clippy::should_panic_without_expect)]
    #[should_panic]
    fn test_tile_renderer_panic(
        w in 16u32..200u32,
        h in 16u32..200u32,
        i0 in 3usize..1000usize,
        i1 in 3usize..1000usize,
        i2 in 3usize..1000usize,
    ) {
        let mut renderer = TileRenderer::new(w, h);
        let verts = vec![
            (Vec3::new(0.0, 0.0, 0.0), 1.0),
            (Vec3::new(1.0, 0.0, 0.0), 1.0),
            (Vec3::new(0.0, 1.0, 0.0), 1.0),
        ];
        // i0, i1, i2 are guaranteed to be >= 3, which is out of bounds for `verts` (len=3)
        let indices = vec![[i0, i1, i2]];

        // This will panic internally in submit_mesh due to:
        // let v0 = vertices[i0];
        renderer.submit_mesh(&indices, &verts, 0xFFFF_FFFF);
    }
}
