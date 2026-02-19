use abrash::texture::Texture;
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::math::{Vec3, Vec2};
use abrash::rasterizer::fill_triangle_textured;

#[test]
fn crash_texture_invariant_violation() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // Create a small texture (2x2) -> 4 pixels
    let mut texture = Texture::new(2, 2).unwrap();

    // Maliciously update width to be huge.
    // The texture buffer still only has 4 pixels.
    // But the rasterizer will think it has 1000 columns.
    texture.width = 1000;

    // Setup a triangle that maps to the texture
    // Vertices at screen coordinates (0,0), (100,0), (0,100)
    // UVs map to (0,0), (1,0), (0,1)
    // The rasterizer will interpolate UVs.
    // At u=0.5, x_tex = 0.5 * 1000 = 500.
    // It will try to access pixel at index 500.
    // Boom.

    let v0 = ((Vec3::new(0.0, 0.0, 1.0), 1.0), Vec2::new(0.0, 0.0));
    let v1 = ((Vec3::new(100.0, 0.0, 1.0), 1.0), Vec2::new(1.0, 0.0));
    let v2 = ((Vec3::new(0.0, 100.0, 1.0), 1.0), Vec2::new(0.0, 1.0));

    fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &texture);
}
