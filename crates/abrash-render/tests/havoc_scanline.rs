use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::flat::draw_scanline_flat;
use abrash_render::zbuffer::ZBuffer;
use proptest::prelude::*;

proptest! {
    #[test]
    #[ignore = "Intentional crash for Havoc chaos engineering"]
    fn havoc_test_draw_scanline_flat_zbuffer_oob_proptest(
        fb_w in 100u32..200, fb_h in 100u32..200,
        zb_w in 10u32..50, zb_h in 10u32..50,
        y in 0i32..100, x_start in 0i32..50, x_end in 51i32..100
    ) {
        let mut fb = Framebuffer::new(fb_w, fb_h).unwrap();
        let mut zb = ZBuffer::new(zb_w, zb_h).unwrap();

        // This will panic when the framebuffer width is used to index into the smaller ZBuffer
        draw_scanline_flat(&mut fb, &mut zb, y, x_start, x_end, 1.0, 0.0, 0xFFFF_FFFF);
    }
}
