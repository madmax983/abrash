use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::{Texture, fill_triangle_textured};
use abrash::tile_renderer::TileRenderer;
use abrash::zbuffer::ZBuffer;

#[test]
fn textured_quad_rendering_matches_scanline() {
    let width = 100;
    let height = 100;

    // Create a 2x2 checkerboard texture
    let mut texture = Texture::new(2, 2).unwrap();
    texture.set_pixel(0, 0, 0xFFFFFFFF); // White
    texture.set_pixel(1, 0, 0xFF000000); // Black
    texture.set_pixel(0, 1, 0xFF000000); // Black
    texture.set_pixel(1, 1, 0xFFFFFFFF); // White

    // Two triangles forming a quad covering the screen
    // V0 (-1, 1) -> (0,0)
    // V1 (-1, -1) -> (0,1)
    // V2 (1, -1) -> (1,1)
    // V3 (1, 1) -> (1,0)

    // Tri 1: V0, V1, V2
    let v0 = (Vec3::new(-1.0, 1.0, 1.0), 1.0);
    let uv0 = Vec2::new(0.0, 0.0);
    let v1 = (Vec3::new(-1.0, -1.0, 1.0), 1.0);
    let uv1 = Vec2::new(0.0, 1.0);
    let v2 = (Vec3::new(1.0, -1.0, 1.0), 1.0);
    let uv2 = Vec2::new(1.0, 1.0);

    // Tri 2: V0, V2, V3
    let v3 = (Vec3::new(1.0, 1.0, 1.0), 1.0);
    let uv3 = Vec2::new(1.0, 0.0);

    let batch = vec![
        (v0, uv0, v1, uv1, v2, uv2),
        (v0, uv0, v2, uv2, v3, uv3),
    ];

    // Tiled rendering
    let mut fb_tiled = Framebuffer::new(width, height).unwrap();
    let mut zb_tiled = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);
    tr.render_batch_textured(&mut fb_tiled, &mut zb_tiled, &texture, &batch);

    // Reference scanline rendering
    let mut fb_ref = Framebuffer::new(width, height).unwrap();
    let mut zb_ref = ZBuffer::new(width, height).unwrap();
    for tri in &batch {
        fill_triangle_textured(
            &mut fb_ref,
            &mut zb_ref,
            (tri.0, tri.1),
            (tri.2, tri.3),
            (tri.4, tri.5),
            &texture,
        );
    }

    // Compare
    let tiled_slice = fb_tiled.as_slice();
    let ref_slice = fb_ref.as_slice();
    let mut mismatches = 0;

    for i in 0..tiled_slice.len() {
        if tiled_slice[i] != ref_slice[i] {
            mismatches += 1;
            if mismatches <= 5 {
                println!(
                    "Pixel mismatch at index {i} (x={}, y={}): tiled={:08X}, ref={:08X}",
                    i % width as usize, i / width as usize, tiled_slice[i], ref_slice[i]
                );
            }
        }
    }

    // Allow a small number of mismatches due to floating point accumulation differences
    // between tile-local calculation and global scanline calculation.
    // 100x100 = 10000 pixels. 100 pixels is 1%.
    assert!(mismatches < 50, "Too many mismatches: {}", mismatches);
}
