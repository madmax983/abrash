use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::circle::{draw_circle, fill_circle};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn test_draw_circle_no_panic(
        xc in -100i32..100,
        yc in -100i32..100,
        radius in (i32::MAX / 2 + 2)..=i32::MAX
    ) {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        draw_circle(&mut fb, xc, yc, radius, 0xFFFF_FFFF);
    }

    #[test]
    fn test_fill_circle_no_panic(
        xc in -100i32..100,
        yc in -100i32..100,
        radius in (i32::MAX / 2 + 2)..=i32::MAX
    ) {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fill_circle(&mut fb, xc, yc, radius, 0xFFFF_FFFF);
    }
}
