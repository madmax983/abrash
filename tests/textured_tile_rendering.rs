use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::{fill_triangle_textured, Texture};
use abrash::tile_renderer::{TexturedClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;

#[test]
fn textured_triangle_rendering_matches_reference() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tr = TileRenderer::new(width, height);

    let mut texture = Texture::new(32, 32).unwrap();
    // Fill texture with a pattern
    for y in 0..32 {
        for x in 0..32 {
            let color = if (x + y) % 2 == 0 { 0xFFFFFFFF } else { 0xFF000000 };
            texture.set_pixel(x, y, color);
        }
    }

    let v0 = (Vec3::new(0.0, 0.5, 5.0), 5.0);
    let uv0 = Vec2::new(0.5, 0.0);

    let v1 = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
    let uv1 = Vec2::new(0.0, 1.0);

    let v2 = (Vec3::new(0.5, -0.5, 5.0), 5.0);
    let uv2 = Vec2::new(1.0, 1.0);

    let tri: TexturedClipTriangle = (v0, uv0, v1, uv1, v2, uv2);

    tr.render_batch_textured(&mut fb, &mut zb, &[tri], &texture);

    // Reference
    let mut fb_ref = Framebuffer::new(width, height).unwrap();
    let mut zb_ref = ZBuffer::new(width, height).unwrap();

    fill_triangle_textured(&mut fb_ref, &mut zb_ref, (v0, uv0), (v1, uv1), (v2, uv2), &texture);

    let ref_pixels = fb_ref.as_slice();
    let tile_pixels = fb.as_slice();

    let mut mismatches = 0;
    for i in 0..(width * height) as usize {
        if tile_pixels[i] != ref_pixels[i] {
            mismatches += 1;
            if mismatches < 10 {
                 println!("Mismatch at {}: tiled={:08X}, ref={:08X}", i, tile_pixels[i], ref_pixels[i]);
            }
        }
    }
    assert_eq!(mismatches, 0, "Found {} pixel mismatches", mismatches);
}
