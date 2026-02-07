#[cfg(feature = "parallel")]
#[test]
#[should_panic(expected = "Framebuffer width mismatch")]
fn security_tile_renderer_buffer_overflow() {
    use abrash::framebuffer::Framebuffer;
    use abrash::tile_renderer::{ClipTriangle, TileRenderer};
    use abrash::zbuffer::ZBuffer;
    use abrash::math::Vec3;

    // 1. Create a TileRenderer with LARGE dimensions
    // 1000x1000 = 1M pixels
    let width = 1000;
    let height = 1000;
    let mut renderer = TileRenderer::new(width, height);

    // 2. Create a Framebuffer with SMALL dimensions
    // 10x10 = 100 pixels
    let mut fb = Framebuffer::new(10, 10).unwrap();
    let mut zb = ZBuffer::new(10, 10).unwrap();

    // 3. Create a triangle that covers some tiles
    // This triangle will be processed by the renderer
    // We need to make sure it actually generates some pixels
    // Coordinates are in clip space (-1 to 1)
    let triangles: Vec<ClipTriangle> = vec![
        ((Vec3::new(-0.5, -0.5, 0.5), 1.0),
         (Vec3::new(0.5, -0.5, 0.5), 1.0),
         (Vec3::new(0.0, 0.5, 0.5), 1.0),
         0xFF0000FF),
    ];

    println!("Havoc: Attempting to write to out-of-bounds memory...");

    // 4. Render!
    // This should cause a panic safely now, instead of a segfault.
    renderer.render_batch(&mut fb, &mut zb, &triangles);

    println!("Havoc: Survived? That's disappointing.");
}
