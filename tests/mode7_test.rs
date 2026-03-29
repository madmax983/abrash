use abrash::framebuffer::Framebuffer;
use abrash::texture::Texture;

#[cfg(feature = "nova")]
#[test]
fn test_mode7_rendering() {
    use abrash::experimental::mode7::{Mode7Config, render_mode7};
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut tex = Texture::new(64, 64).unwrap();

    // Fill texture with white
    for y in 0..64 {
        for x in 0..64 {
            tex.set_pixel(x, y, 0xFFFFFFFF);
        }
    }

    let config = Mode7Config {
        cy: 50.0,
        fov: 100.0,
        horizon: 50.0,
        ..Default::default()
    };

    render_mode7(&mut fb, &tex, &config);

    // Check that some pixels below the horizon were drawn
    let mut drawn_pixels = 0;
    for y in 51..100 {
        for x in 0..100 {
            if fb.get_pixel(x as i32, y as i32) == Some(0xFFFFFFFF) {
                drawn_pixels += 1;
            }
        }
    }

    assert!(
        drawn_pixels > 0,
        "Mode7 did not render any pixels below horizon"
    );
}
