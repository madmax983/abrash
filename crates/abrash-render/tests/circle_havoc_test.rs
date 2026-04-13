use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_circle, fill_circle};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn havoc_circle_overflow(xc in any::<i32>(), yc in any::<i32>(), radius in (i32::MAX / 2 + 2)..=i32::MAX) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        draw_circle(&mut fb, xc, yc, radius, 0xFFFFFFFF);
        fill_circle(&mut fb, xc, yc, radius, 0xFFFFFFFF);
    }
}
