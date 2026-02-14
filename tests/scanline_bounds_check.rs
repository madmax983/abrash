
#[cfg(test)]
mod tests {
    use abrash::framebuffer::Framebuffer;
    use abrash::zbuffer::ZBuffer;
    use abrash::rasterizer::draw_scanline_flat;

    #[test]
    fn test_draw_scanline_out_of_bounds_y() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // y = 200 is out of bounds. Should return safely (no panic/crash/UB).
        draw_scanline_flat(&mut fb, &mut zb, 200, 0, 50, 0.0, 0.0, 0xFFFFFFFF);

        // If we reach here, it didn't crash.
        assert!(true);
    }
}
