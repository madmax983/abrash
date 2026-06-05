use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::ellipse::{draw_ellipse, fill_ellipse};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_draw_ellipse_extreme_no_panic(xc in any::<i32>(), yc in any::<i32>(), rx in any::<i32>(), ry in any::<i32>(), fb_w in 1..2000u32, fb_h in 1..2000u32) {
        if let Ok(mut fb) = Framebuffer::new(fb_w, fb_h) {
            draw_ellipse(&mut fb, xc, yc, rx, ry, 0xFFFF_FFFF);
        }
    }

    #[test]
    fn test_fill_ellipse_extreme_no_panic(xc in any::<i32>(), yc in any::<i32>(), rx in any::<i32>(), ry in any::<i32>(), fb_w in 1..2000u32, fb_h in 1..2000u32) {
        if let Ok(mut fb) = Framebuffer::new(fb_w, fb_h) {
            fill_ellipse(&mut fb, xc, yc, rx, ry, 0xFFFF_FFFF);
        }
    }
}
