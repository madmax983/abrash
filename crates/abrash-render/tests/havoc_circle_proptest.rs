use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::circle::{draw_circle, fill_circle};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_circle_overflow_panic(xc in any::<i32>(), yc in any::<i32>(), radius in (i32::MAX / 2 + 2)..=i32::MAX) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Just calling one of them is enough to trigger the panic for the PR
        fill_circle(&mut fb, xc, yc, radius, 0xFFFF_FFFF);
    }
}
