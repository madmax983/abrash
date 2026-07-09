use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::rect::{draw_rect, draw_rounded_rect, fill_rect, fill_rounded_rect};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_draw_rect_havoc(
        x in i32::MIN..i32::MAX, y in i32::MIN..i32::MAX, w in 0..u32::MAX, h in 0..u32::MAX,
        color in 0..u32::MAX,
    ) {
        let mut fb = Framebuffer::new(320, 240).unwrap();
        draw_rect(&mut fb, x, y, w, h, color);
    }

    #[test]
    fn test_fill_rect_havoc(
        x in i32::MIN..i32::MAX, y in i32::MIN..i32::MAX, w in 0..u32::MAX, h in 0..u32::MAX,
        color in 0..u32::MAX,
    ) {
        let mut fb = Framebuffer::new(320, 240).unwrap();
        fill_rect(&mut fb, x, y, w, h, color);
    }

    #[test]
    fn test_draw_rounded_rect_havoc(
        x in i32::MIN..i32::MAX, y in i32::MIN..i32::MAX, w in 0..u32::MAX, h in 0..u32::MAX,
        r in i32::MIN..i32::MAX, color in 0..u32::MAX,
    ) {
        let mut fb = Framebuffer::new(320, 240).unwrap();
        draw_rounded_rect(&mut fb, x, y, w, h, r, color);
    }

    #[test]
    fn test_fill_rounded_rect_havoc(
        x in i32::MIN..i32::MAX, y in i32::MIN..i32::MAX, w in 0..u32::MAX, h in 0..u32::MAX,
        r in i32::MIN..i32::MAX, color in 0..u32::MAX,
    ) {
        let mut fb = Framebuffer::new(320, 240).unwrap();
        fill_rounded_rect(&mut fb, x, y, w, h, r, color);
    }
}
