use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::TileRenderer;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_tile_renderer_gouraud_render() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
    let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
    let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
    let c0 = Vec3::new(1.0, 0.0, 0.0);
    let c1 = Vec3::new(0.0, 1.0, 0.0);
    let c2 = Vec3::new(0.0, 0.0, 1.0);

    let triangles = vec![((v0, c0), (v1, c1), (v2, c2))];

    fb.clear(0xFF00_0000);
    zb.clear();

    // Should not panic, and should render pixels
    renderer.render_batch_gouraud(&mut fb, &mut zb, &triangles);

    // Verify center pixel is set (approx blended color)
    let center = fb.get_pixel(50, 50);
    assert!(center.is_some());
    let color = center.unwrap();
    assert_ne!(color, 0xFF00_0000, "Pixel should not be background black");
}
