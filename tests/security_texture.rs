use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::texture::{Texture, FilterMode};
use abrash::rasterizer::fill_triangle_textured;
use abrash::math::{Vec3, Vec2};

#[test]
#[should_panic(expected = "Texture buffer too small")]
fn test_malformed_texture_panics() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Create a malformed texture manually
    // We create a valid one, then sabotage it.
    let mut texture = Texture::new(100, 100).unwrap();
    // Intentionally truncate the buffer to provoke the assertion.
    texture.pixels.truncate(10);

    // Valid triangle that would normally render fine
    let v0 = ((Vec3::new(-0.5, -0.5, 5.0), 5.0), Vec2::new(0.0, 0.0));
    let v1 = ((Vec3::new(0.5, -0.5, 5.0), 5.0), Vec2::new(1.0, 0.0));
    let v2 = ((Vec3::new(0.0, 0.5, 5.0), 5.0), Vec2::new(0.5, 1.0));

    // This should panic now, instead of running into UB
    fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &texture);
}
