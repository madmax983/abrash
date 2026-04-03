use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::rasterizer::fill_triangle_3d;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    #[ignore = "👹 Havoc: Triggering panics in flat triangle rasterizer via f32 extremes"]
    fn test_triangle_torture(
        v0x in any::<f32>(), v0y in any::<f32>(), v0z in any::<f32>(), v0w in any::<f32>(),
        v1x in any::<f32>(), v1y in any::<f32>(), v1z in any::<f32>(), v1w in any::<f32>(),
        v2x in any::<f32>(), v2y in any::<f32>(), v2z in any::<f32>(), v2w in any::<f32>(),
    ) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let mut zb = ZBuffer::new(100, 100).unwrap();

        fill_triangle_3d(&mut fb, &mut zb,
            (Vec3::new(v0x, v0y, v0z), v0w),
            (Vec3::new(v1x, v1y, v1z), v1w),
            (Vec3::new(v2x, v2y, v2z), v2w),
            0xFFFF_FFFF
        );
    }
}
