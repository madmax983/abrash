use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_circle, fill_circle};
use proptest::prelude::*;

proptest! {
    #[test]
    #[should_panic(expected = "attempt to multiply with overflow")]
    fn test_draw_circle_fuzz(r in 536894084..=i32::MAX) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        draw_circle(&mut fb, 0, 0, r, 0xFFFFFFFF);
    }

    #[test]
    #[should_panic(expected = "attempt to multiply with overflow")]
    fn test_fill_circle_fuzz(r in 536894084..=i32::MAX) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fill_circle(&mut fb, 0, 0, r, 0xFFFFFFFF);
    }
}
