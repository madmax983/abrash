#![allow(missing_docs)]
use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_circle, fill_circle};
use proptest::prelude::*;

proptest! {
    #[test]
    fn havoc_circle_proptest(xc in any::<i32>(), yc in any::<i32>(), radius in any::<i32>()) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        draw_circle(&mut fb, xc, yc, radius, 0xFFFF_FFFF);
        fill_circle(&mut fb, xc, yc, radius, 0xFFFF_FFFF);
    }
}
