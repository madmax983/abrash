use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::ClipTriangle;
use abrash::rasterizer::tile::TileRenderer;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_havoc_prepared_triangle_i16_overflow() {
    let width = 100;
    let height = 100;

    let mut renderer = TileRenderer::new(width, height);

    // X overflows i16 and wraps to a small positive number within bounds!
    // 32768 is i16::MIN (-32768).
    // We want it to wrap to +50.
    // 50 = X as i16.
    // X = 50 + 65536 = 65586

    let base_x = 65586.0;

    let v0 = (Vec3::new(base_x, 10.0, 0.5), 1.0);
    let v1 = (Vec3::new(base_x + 10.0, 10.0, 0.5), 1.0);
    let v2 = (Vec3::new(base_x, 20.0, 0.5), 1.0);

    let tris: Vec<ClipTriangle> = vec![(v0, v1, v2, 0xFFFFFFFF)];

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Fill with black
    fb.clear(0xFF000000);

    renderer.render_batch(&mut fb, &mut zb, &tris);

    // We expect the triangle to NOT be rendered since it's far outside the width (65586 > 100).
    // BUT because of i16 overflow, it wraps to 50! And renders in the middle of the screen!
    let mut rendered_pixels = 0;
    for y in 0..height {
        for x in 0..width {
            if fb.get_pixel(x as i32, y as i32).unwrap() == 0xFFFFFFFF {
                rendered_pixels += 1;
            }
        }
    }

    // This asserts that Havoc found the bug: a triangle way off screen wrapped and rendered on screen!
    assert!(
        rendered_pixels > 0,
        "Triangle with X=65586 wrapped around and rendered on screen!"
    );
}
