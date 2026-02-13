use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
use proptest::prelude::*;

proptest! {
    // Run enough cases to hit edge cases
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn fuzz_rasterizer_safety(
        v0_x in any::<f32>(), v0_y in any::<f32>(), v0_z in any::<f32>(), v0_w in any::<f32>(),
        v1_x in any::<f32>(), v1_y in any::<f32>(), v1_z in any::<f32>(), v1_w in any::<f32>(),
        v2_x in any::<f32>(), v2_y in any::<f32>(), v2_z in any::<f32>(), v2_w in any::<f32>(),
    ) {
        // We use a small framebuffer to make it faster/easier to hit bounds
        let width = 64;
        let height = 64;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        let v0 = (Vec3::new(v0_x, v0_y, v0_z), v0_w);
        let v1 = (Vec3::new(v1_x, v1_y, v1_z), v1_w);
        let v2 = (Vec3::new(v2_x, v2_y, v2_z), v2_w);
        let color = 0xFFFFFFFF;

        // This should not panic, crash, or cause OOB access.
        // Even with NaN, Infinity, Subnormal, or Max values.
        fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
    }
}
