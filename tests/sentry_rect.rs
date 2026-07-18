#[cfg(test)]
mod tests {
    use abrash_core::framebuffer::Framebuffer;
    use abrash_render::rasterizer::rect::*;

    #[test]
    fn test_draw_vertical_line_unchecked_oob() {
        let mut fb = Framebuffer::new(10, 10).unwrap();

        draw_rect(&mut fb, -5, 5, 2, 2, 0xFFFF_FFFF);
        draw_rect(&mut fb, 5, -5, 2, 2, 0xFFFF_FFFF);
        draw_rect(&mut fb, 5, 15, 2, 2, 0xFFFF_FFFF);
        draw_rect(&mut fb, 15, 5, 2, 2, 0xFFFF_FFFF);

        // draw_rounded_rect
        draw_rounded_rect(&mut fb, -5, 5, 2, 2, 1, 0xFFFF_FFFF);
    }
}
