use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;
use abrash_render::experimental::tunnel::apply_tunnel;

#[test]
fn test_tunnel_rendering_output() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut texture = Texture::new(64, 64).unwrap();

    // Fill texture with white
    for y in 0..64 {
        for x in 0..64 {
            texture.set_pixel(x, y, 0xFF_FF_FF_FF);
        }
    }

    apply_tunnel(&mut fb, 0.0, &texture);

    // Center pixel should be drawn to (it uses fallback distance due to math)
    assert!(fb.get_pixel(50, 50).unwrap() != 0);
    // Boundary pixel should be drawn to
    assert!(fb.get_pixel(10, 10).unwrap() != 0);
}
