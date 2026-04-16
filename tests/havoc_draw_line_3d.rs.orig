use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::rasterizer::line::draw_line_3d;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    #[ignore = "👺 Havoc: Intentionally triggers UB/Panic by bypassing dimensional guards. Cannot be caught nicely in proptest due to SIGABRT"]
    fn havoc_test_draw_line_3d_zbuffer_oob(
        width in 100u32..200u32,
        height in 100u32..200u32,
        zb_width in 1u32..10u32,
        zb_height in 1u32..10u32,
    ) {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(zb_width, zb_height).unwrap();

        let v0 = (Vec3::new(-1.0, -1.0, 0.5), 1.0);
        let v1 = (Vec3::new(1.0, 1.0, 0.5), 1.0);

        // This will cause an out of bounds slice access when writing to the Z-Buffer
        // We ignore it so it doesn't kill the cargo test runner during normal suite runs.
        draw_line_3d(&mut fb, &mut zb, v0, v1, 0xFFFFFFFF);
    }
}
