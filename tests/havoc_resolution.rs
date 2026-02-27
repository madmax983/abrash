use abrash::rasterizer::{TileRenderer, ClipTriangle};
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::math::Vec3;

#[test]
fn test_havoc_resolution_overflow() {
    let width = 65536;
    let height = 32;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    // Triangle at far right, exceeding i16 range (32767)
    // NDC x=0.8 maps to approx x=58982
    let tri: ClipTriangle = (
        (Vec3::new(0.8, 0.0, 1.0), 1.0),
        (Vec3::new(0.9, 0.0, 1.0), 1.0),
        (Vec3::new(0.8, 0.9, 1.0), 1.0), // Make it tall enough to definitely hit a scanline
        0xFF0000FF // Red
    );

    // If i16 overflow happens in PreparedTriangle::aabb_min_x,
    // the triangle coordinates might wrap to negative values.
    // This would cause it to be binned to tile 0 (or skipped),
    // and thus NOT drawn at the correct location (far right).
    renderer.render_batch(&mut fb, &mut zb, &[tri]);

    // Check if any pixels were drawn in the expected region
    // Region: x ~ 58982.
    // Let's check the whole framebuffer.
    let mut drawn_pixels = 0;
    for &p in fb.as_slice() {
        if p != 0xFF000000 { // Default clear color is Black (0xFF000000) for TileRenderer?
            // Wait, TileRenderer clears to 0xFF000000.
            // My triangle is 0xFF0000FF.
            drawn_pixels += 1;
        }
    }

    assert!(drawn_pixels > 0, "Triangle was culled or lost due to integer overflow!");
}
