use abrash::texture::{FilterMode, Texture};

#[test]
fn test_bilinear_filtering() {
    let mut texture = Texture::new(2, 2);
    // (0,0) = Black
    // (1,0) = White
    // (0,1) = Black
    // (1,1) = White
    // Vertical stripes
    texture.set_pixel(0, 0, 0xFF000000);
    texture.set_pixel(1, 0, 0xFFFFFFFF);
    texture.set_pixel(0, 1, 0xFF000000);
    texture.set_pixel(1, 1, 0xFFFFFFFF);

    texture.filter_mode = FilterMode::Bilinear;

    // Sample exactly between the two columns (u=0.5)
    // In normalized coords [0,1], pixel centers are at:
    // x=0: 0.25 (0 to 0.5)
    // x=1: 0.75 (0.5 to 1.0)

    // If we sample at u=0.5, we are exactly on the boundary.
    // Bilinear should interpolate between left (Black) and right (White).
    // Expected: Grey (0xFF7F7F7F or 0xFF808080)

    let color = texture.get_pixel(0.5, 0.5);

    let r = (color >> 16) & 0xFF;
    let g = (color >> 8) & 0xFF;
    let b = color & 0xFF;

    // Check that we are roughly grey (127 or 128)
    // If it was Nearest, it would be either 0 or 255.
    assert!(
        r > 100 && r < 155,
        "Red component {} should be around 128",
        r
    );
    assert!(
        g > 100 && g < 155,
        "Green component {} should be around 128",
        g
    );
    assert!(
        b > 100 && b < 155,
        "Blue component {} should be around 128",
        b
    );
}
