use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::{Texture, fill_triangle_textured};
use abrash::tile_renderer::TileRenderer;
use abrash::zbuffer::ZBuffer;

#[test]
fn textured_tile_rendering_matches_scanline() {
    let width = 100;
    let height = 100;

    // Create a simple checkerboard texture
    let texture = Texture::checkered(16, 16, 0xFFFFFFFF, 0xFF000000).unwrap();

    // Triangle covering the center of the screen
    let v0 = ((Vec3::new(0.0, 0.5, 5.0), 5.0), Vec2::new(0.5, 0.0));
    let v1 = ((Vec3::new(-0.5, -0.5, 5.0), 5.0), Vec2::new(0.0, 1.0));
    let v2 = ((Vec3::new(0.5, -0.5, 5.0), 5.0), Vec2::new(1.0, 1.0));

    // Reference: scanline renderer
    let mut fb_ref = Framebuffer::new(width, height).unwrap();
    let mut zb_ref = ZBuffer::new(width, height).unwrap();
    fill_triangle_textured(&mut fb_ref, &mut zb_ref, v0, v1, v2, &texture);

    // Tiled renderer
    let mut fb_tile = Framebuffer::new(width, height).unwrap();
    let mut zb_tile = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);

    tr.render_batch_textured(&mut fb_tile, &mut zb_tile, &[(v0.0, v0.1, v1.0, v1.1, v2.0, v2.1)], &texture);

    // Compare pixels
    let ref_pixels = fb_ref.as_slice();
    let tile_pixels = fb_tile.as_slice();

    for i in 0..(width * height) as usize {
        assert_eq!(
            tile_pixels[i], ref_pixels[i],
            "Pixel mismatch at index {i} (x={}, y={}): tiled=0x{:08X}, ref=0x{:08X}",
            i % width as usize,
            i / width as usize,
            tile_pixels[i],
            ref_pixels[i]
        );
    }
}
