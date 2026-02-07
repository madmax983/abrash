#[cfg(test)]
mod tests {
    use abrash::framebuffer::Framebuffer;
    use abrash::zbuffer::ZBuffer;
    use abrash::tile_renderer::TileRenderer;
    use abrash::math::Vec3;

    #[test]
    #[should_panic]
    fn test_tile_renderer_dimension_mismatch() {
        // Create a TileRenderer expecting 100x100
        let mut tr = TileRenderer::new(100, 100);

        // Create a smaller Framebuffer (10x10)
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut zb = ZBuffer::new(10, 10).unwrap();

        // Create a triangle that covers the screen
        let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
        let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
        let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
        let color = 0xFFFF_0000;

        // This should panic safely, but currently it might cause UB/Crash if parallel feature is on,
        // or just out of bounds access if using unchecked methods.
        // Even with sequential rendering, merge_tile_direct uses fb_slice[...].copy_from_slice(...)
        // which might panic if out of bounds, preventing UB in safe code.
        // But let's see.
        tr.render_batch(&mut fb, &mut zb, &[(v0, v1, v2, color)]);
    }
}
