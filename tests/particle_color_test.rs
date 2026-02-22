use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::particles::{Particle, ParticleSystem};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_particle_color_rendering() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create a pure white texture
    let mut texture = Texture::new(2, 2).unwrap();
    texture.pixels.fill(0xFFFFFFFF);

    let mut sys = ParticleSystem::new(10, texture);

    // Add a RED particle at origin
    // 0xFFFF0000: Alpha=255, R=255, G=0, B=0
    sys.particles.push(Particle::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
        1.0, // Life
        5.0, // Size (big enough to cover center)
        0xFFFF0000,
    ));

    // Setup camera looking at origin
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 10.0), // Eye
        Vec3::new(0.0, 0.0, 0.0),  // Target
        Vec3::new(0.0, 1.0, 0.0),  // Up
    );
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);

    // Clear FB to Blue to distinguish from default black or texture white
    fb.clear(0xFF0000FF);
    zb.clear();

    sys.render(&mut fb, &mut zb, view, proj);

    // Check center pixel
    let center_pixel = unsafe { fb.get_pixel_unchecked(50, 50) };

    // Expected: Red (0xFFFF0000)
    // Actual (Current): White (0xFFFFFFFF) because color is ignored
    assert_eq!(
        center_pixel, 0xFFFF0000,
        "Expected Red pixel 0xFFFF0000, got 0x{:08X}",
        center_pixel
    );
}
