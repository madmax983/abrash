#[cfg(test)]
mod tests {
    use abrash::framebuffer::Framebuffer;
    use abrash::zbuffer::ZBuffer;
    use abrash::math::Vec3;
    use abrash::rasterizer::fill_triangle_3d;

    #[test]
    fn test_scanline_overflow_panic() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Construct a triangle that projects to X coordinates causing overflow in draw_scanline_flat
        // but stays within valid range for EdgeWalker z-interpolation (24.8 fixed point limit ~8 million)

        // Target screen X approx -2e9 (near i32::MIN)
        // Screen width 100 -> half width 50.
        // NDC X = -2e9 / 50 = -4e7.
        // v0.x = -4e7, w=1.0.
        let v0 = (Vec3::new(-40_000_000.0, 0.0, 0.0), 1.0);

        // v1 at center screen, z = 5,000,000 (fits in 24.8 fixed point range max ~8.3M)
        let v1 = (Vec3::new(0.0, 0.0, 5_000_000.0), 1.0);

        // v2 just to form a triangle
        let v2 = (Vec3::new(0.0, 10.0, 0.0), 1.0);

        let color = 0xFFFF_0000;

        // This should not panic with the fix.
        // Without fix, xs (approx -2e9) * dz_dx_fixed (approx 1) causes subtraction of huge value from z_fixed,
        // likely overflowing if z_fixed is small.
        fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, color);
    }
}
