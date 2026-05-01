use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::rect::{draw_rect, fill_rounded_rect};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_draw_rect_extreme_no_panic(x in i32::MIN..i32::MAX, y in i32::MIN..i32::MAX, w in 0..u32::MAX, h in 0..u32::MAX, fb_w in 1..2000u32, fb_h in 1..2000u32) {
        if let Ok(mut fb) = Framebuffer::new(fb_w, fb_h) {
            draw_rect(&mut fb, x, y, w, h, 0xFFFFFFFF);
        }
    }

    #[test]
    fn test_fill_rounded_rect_extreme_no_panic(x in i32::MIN..i32::MAX, y in i32::MIN..i32::MAX, w in 0..u32::MAX, h in 0..u32::MAX, r in i32::MIN..i32::MAX, fb_w in 1..2000u32, fb_h in 1..2000u32) {
        if let Ok(mut fb) = Framebuffer::new(fb_w, fb_h) {
            fill_rounded_rect(&mut fb, x, y, w, h, r, 0xFFFFFFFF);
        }
    }
}
