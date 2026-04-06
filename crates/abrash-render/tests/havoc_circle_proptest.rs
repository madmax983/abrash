use proptest::prelude::*;
use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_circle, fill_circle};

proptest! {
    #[test]
    fn test_draw_circle_havoc(
        xc in any::<i32>(),
        yc in any::<i32>(),
        radius in (i32::MAX / 2 + 2)..=i32::MAX
    ) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        draw_circle(&mut fb, xc, yc, radius, 0xFFFFFFFF);
    }

    #[test]
    fn test_fill_circle_havoc(
        xc in any::<i32>(),
        yc in any::<i32>(),
        radius in (i32::MAX / 2 + 2)..=i32::MAX
    ) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fill_circle(&mut fb, xc, yc, radius, 0xFFFFFFFF);
    }
}
