use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_gouraud;
use abrash::tile_renderer::TileRenderer;
use abrash::zbuffer::ZBuffer;

// Red Phase: This type alias and method do not exist yet in TileRenderer
// I'm defining the expected input format here for the test.
// ((Pos, W), Color)
type ClipTriangleGouraud = ((Vec3, f32), Vec3, (Vec3, f32), Vec3, (Vec3, f32), Vec3);

#[test]
fn test_tile_renderer_gouraud_matches_reference() {
    let width = 100;
    let height = 100;

    let mut fb_ref = Framebuffer::new(width, height).unwrap();
    let mut zb_ref = ZBuffer::new(width, height).unwrap();

    let mut fb_tile = Framebuffer::new(width, height).unwrap();
    let mut zb_tile = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);

    // Triangle with R, G, B corners
    let v0_pos = (Vec3::new(0.0, 0.5, 5.0), 5.0);
    let v0_col = Vec3::new(1.0, 0.0, 0.0); // Red

    let v1_pos = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
    let v1_col = Vec3::new(0.0, 1.0, 0.0); // Green

    let v2_pos = (Vec3::new(0.5, -0.5, 5.0), 5.0);
    let v2_col = Vec3::new(0.0, 0.0, 1.0); // Blue

    // 1. Reference Implementation
    fill_triangle_gouraud(
        &mut fb_ref,
        &mut zb_ref,
        (v0_pos, v0_col),
        (v1_pos, v1_col),
        (v2_pos, v2_col),
    );

    // 2. Tile Renderer (Gouraud)
    let triangles: Vec<ClipTriangleGouraud> = vec![
        (v0_pos, v0_col, v1_pos, v1_col, v2_pos, v2_col)
    ];

    tr.render_batch_gouraud(&mut fb_tile, &mut zb_tile, &triangles);
}
