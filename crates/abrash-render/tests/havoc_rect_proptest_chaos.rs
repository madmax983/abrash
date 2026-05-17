use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::rect::{draw_rounded_rect, fill_rounded_rect};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_draw_rounded_rect_extreme_no_panic(
        x in any::<i32>(),
        y in any::<i32>(),
        w in any::<u32>(),
        h in any::<u32>(),
        r in any::<i32>(),
        fb_w in 1..=2000u32,
        fb_h in 1..=2000u32
    ) {
        if let Ok(mut fb) = Framebuffer::new(fb_w, fb_h) {
            draw_rounded_rect(&mut fb, x, y, w, h, r, 0xFFFF_FFFF);
        }
    }

    #[test]
    fn test_fill_rounded_rect_extreme_no_panic(
        x in any::<i32>(),
        y in any::<i32>(),
        w in any::<u32>(),
        h in any::<u32>(),
        r in any::<i32>(),
        fb_w in 1..=2000u32,
        fb_h in 1..=2000u32
    ) {
        if let Ok(mut fb) = Framebuffer::new(fb_w, fb_h) {
            fill_rounded_rect(&mut fb, x, y, w, h, r, 0xFFFF_FFFF);
        }
    }
}
