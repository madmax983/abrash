use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured_gouraud;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_textured_gouraud_rendering() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create a simple 2x2 white texture
    let texture = Texture::checkered(2, 2, 0xFFFFFFFF, 0xFFFFFFFF).unwrap();

    // Triangle
    // v0: Top, Red, UV(0.5, 0)
    let v0 = (
        (Vec3::new(0.0, 0.5, -0.5), 1.0), // Pos, w
        Vec3::new(1.0, 0.0, 0.0),         // Color (Red)
        Vec2::new(0.5, 0.0),              // UV
    );

    // v1: Bottom Left, Green, UV(0, 1)
    let v1 = (
        (Vec3::new(-0.8, -0.8, -0.5), 1.0),
        Vec3::new(0.0, 1.0, 0.0), // Green
        Vec2::new(0.0, 1.0),
    );

    // v2: Bottom Right, Blue, UV(1, 1)
    let v2 = (
        (Vec3::new(0.8, -0.8, -0.5), 1.0),
        Vec3::new(0.0, 0.0, 1.0), // Blue
        Vec2::new(1.0, 1.0),
    );

    fill_triangle_textured_gouraud(&mut fb, &mut zb, v0, v1, v2, &texture);

    let checksum = calculate_checksum(&fb);
    println!("Checksum: {checksum:016X}");

    // Placeholder checksum
    assert_eq!(checksum, 0x99602810C9602FB9, "Checksum mismatch!");
}

fn calculate_checksum(fb: &Framebuffer) -> u64 {
    let mut sum: u64 = 0;
    for (i, &pixel) in fb.as_slice().iter().enumerate() {
        if pixel != 0xFF00_0000 {
            sum = sum.wrapping_mul(1_099_511_628_211);
            sum ^= u64::from(pixel);
            sum ^= i as u64;
        }
    }
    sum
}
