use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn crash_texture_invariant_violation() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Create a small texture (2x2) -> 4 pixels
    let texture = Texture::new(2, 2).unwrap();

    // VULNERABILITY FIX:
    // The following line causes a compile error because `width` is now private.
    // This prevents the invariant violation (width > pixels.len()) that caused the crash.
    // texture.width = 1000;

    // Verify safe rendering still works
    let v0 = ((Vec3::new(0.0, 0.0, 1.0), 1.0), Vec2::new(0.0, 0.0));
    let v1 = ((Vec3::new(100.0, 0.0, 1.0), 1.0), Vec2::new(1.0, 0.0));
    let v2 = ((Vec3::new(0.0, 100.0, 1.0), 1.0), Vec2::new(0.0, 1.0));

    fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &texture);
}
