use abrash::experimental::procedural::*;

#[test]
fn test_xor_pattern() {
    let tex = xor_pattern(256, 256).unwrap();
    assert_eq!(tex.width, 256);
    assert_eq!(tex.height, 256);
    // Check specific pixel
    // 0^0 = 0 -> Black
    assert_eq!(tex.get_pixel_texel(0, 0) & 0xFFFFFF, 0);
    // 10^10 = 0 -> Black
    assert_eq!(tex.get_pixel_texel(10, 10) & 0xFFFFFF, 0);
}

#[test]
fn test_grid_pattern() {
    let tex = grid_pattern(100, 100, 10, 0xFFFFFFFF, 0xFF000000).unwrap();
    assert_eq!(tex.width, 100);
    // (0,0) should be line color (white)
    assert_eq!(tex.get_pixel_texel(0, 0), 0xFFFFFFFF);
    // (5,5) should be bg color (black)
    assert_eq!(tex.get_pixel_texel(5, 5), 0xFF000000);
}

#[test]
fn test_white_noise() {
    let tex1 = white_noise(64, 64, 12345).unwrap();
    let tex2 = white_noise(64, 64, 12345).unwrap();

    // Deterministic check
    assert_eq!(tex1.pixels, tex2.pixels);

    // Different seeds
    let tex3 = white_noise(64, 64, 54321).unwrap();
    assert_ne!(tex1.pixels, tex3.pixels);
}

#[test]
fn test_plasma() {
    let tex = plasma(128, 128).unwrap();
    assert_eq!(tex.width, 128);
    // Just ensure it generates *something* (not all black)
    let mut non_black = false;
    for &p in &tex.pixels {
        if (p & 0xFFFFFF) != 0 {
            non_black = true;
            break;
        }
    }
    assert!(non_black, "Plasma texture should not be all black");
}

#[test]
fn test_grid_zero_cell_size() {
    let res = grid_pattern(100, 100, 0, 0xFFFFFFFF, 0xFF000000);
    assert!(res.is_err());
}
