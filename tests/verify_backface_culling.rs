use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_fill_triangle_textured_culling() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut texture = Texture::new(32, 32).unwrap();
    texture.pixels_mut().fill(0xFFFF_FFFF); // White Opaque

    // Center of screen (50, 50).
    // v0 at (0,0,0) -> Proj: (0+1)*50 = 50. (1-0)*50 = 50. -> (50, 50)
    let v_center = ((Vec3::new(0.0, 0.0, 0.0), 1.0), Vec2::default());

    // v1 at (1.0, 0.0, 0.0). X=1.
    // Proj: (1+1)*50 = 100. -> (100, 50)
    let v_right = ((Vec3::new(1.0, 0.0, 0.0), 1.0), Vec2::default());

    // v2 at (0.0, 1.0, 0.0). Y=1.
    // Proj: (1-1)*50 = 0. -> (50, 0)
    let v_top = ((Vec3::new(0.0, 1.0, 0.0), 1.0), Vec2::default());

    // Case 1: Front Facing (should be drawn)
    // Vertices: Center -> Right -> Top
    // Screen: (50,50) -> (100,50) -> (50,0)
    // Edge 1: (50, 0)
    // Edge 2: (-50, -50)
    // Cross: 50*(-50) - 0*(-50) = -2500 < 0.
    // is_backface returns false (keep).

    fb.clear(0);
    fill_triangle_textured(&mut fb, &mut zb, v_center, v_right, v_top, &texture);

    // Check pixel at (60, 40) - inside the triangle
    let p = fb.get_pixel(60, 40).unwrap();
    assert_eq!(
        p, 0xFFFF_FFFF,
        "Front face (Center->Right->Top) should be drawn"
    );

    // Case 2: Back Facing (should be culled)
    // Vertices: Center -> Top -> Right
    // Screen: (50,50) -> (50,0) -> (100,50)
    // Edge 1: (0, -50)
    // Edge 2: (50, 50)
    // Cross: 0*50 - (-50)*50 = 2500 > 0.
    // is_backface returns true (cull).

    fb.clear(0);
    fill_triangle_textured(&mut fb, &mut zb, v_center, v_top, v_right, &texture);
    let p_back = fb.get_pixel(60, 40).unwrap();
    assert_eq!(p_back, 0, "Back face (Center->Top->Right) should be culled");
}
