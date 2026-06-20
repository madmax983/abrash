use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::rect::{draw_rect, draw_rounded_rect, fill_rect, fill_rounded_rect};
use proptest::prelude::*;

proptest! {
    #[test]
    fn havoc_rect_fuzz(
        x in any::<i32>(),
        y in any::<i32>(),
        width in any::<u32>(),
        height in any::<u32>(),
        radius in any::<i32>(),
        fb_w in 1u32..1024u32,
        fb_h in 1u32..1024u32,
    ) {
        if let Ok(mut fb) = Framebuffer::new(fb_w, fb_h) {
            draw_rect(&mut fb, x, y, width, height, 0xFFFF_FFFF);
            fill_rect(&mut fb, x, y, width, height, 0xFFFF_FFFF);
            draw_rounded_rect(&mut fb, x, y, width, height, radius, 0xFFFF_FFFF);
            fill_rounded_rect(&mut fb, x, y, width, height, radius, 0xFFFF_FFFF);
        }
    }
}
