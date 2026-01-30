use abrash::math::Vec2;
use abrash::texture::{Texture, interpolate_uv_perspective};

#[test]
fn test_texture_new() {
    let tex = Texture::new(64, 64);
    assert_eq!(tex.width(), 64);
    assert_eq!(tex.height(), 64);
}

#[test]
fn test_texture_set_get_pixel() {
    let mut tex = Texture::new(4, 4);
    tex.set_pixel(1, 1, 0xFF0000FF); // Red
    assert_eq!(tex.get_pixel(1, 1), 0xFF0000FF);
}

#[test]
fn test_texture_get_pixel_wrapping_positive() {
    let mut tex = Texture::new(4, 4);
    tex.set_pixel(1, 2, 0x00FF00FF); // Green

    // Access beyond bounds should wrap
    assert_eq!(tex.get_pixel(5, 2), 0x00FF00FF); // 5 % 4 = 1
    assert_eq!(tex.get_pixel(1, 6), 0x00FF00FF); // 6 % 4 = 2
}

#[test]
fn test_texture_get_pixel_wrapping_negative() {
    let mut tex = Texture::new(4, 4);
    tex.set_pixel(3, 2, 0x0000FFFF); // Blue

    // Negative indices should wrap
    assert_eq!(tex.get_pixel(-1, 2), 0x0000FFFF); // -1 % 4 = 3
    assert_eq!(tex.get_pixel(3, -2), 0x0000FFFF); // -2 % 4 = 2
}

#[test]
fn test_texture_default_color() {
    let tex = Texture::new(2, 2);
    // Default should be opaque black (0xFF000000 in ARGB format)
    let pixel = tex.get_pixel(0, 0);
    assert_eq!(pixel, 0xFF000000);
}

// TASK 3: Nearest-neighbor sampling
#[test]
fn test_sample_nearest_basic() {
    let mut tex = Texture::new(4, 4);
    tex.set_pixel(0, 0, 0xFF0000FF); // Top-left: Red
    tex.set_pixel(3, 3, 0x00FF00FF); // Bottom-right: Green

    // UV (0, 0) should sample top-left
    assert_eq!(tex.sample_nearest(0.0, 0.0), 0xFF0000FF);

    // UV (0.99, 0.99) should sample bottom-right
    assert_eq!(tex.sample_nearest(0.99, 0.99), 0x00FF00FF);
}

#[test]
fn test_sample_nearest_wrapping() {
    let mut tex = Texture::new(2, 2);
    tex.set_pixel(1, 1, 0x0000FFFF); // Blue at (1, 1)

    // UV coordinates > 1.0 should wrap
    assert_eq!(tex.sample_nearest(1.5, 1.5), 0x0000FFFF);
    assert_eq!(tex.sample_nearest(2.5, 2.5), 0x0000FFFF);

    // Negative UV coordinates should wrap
    assert_eq!(tex.sample_nearest(-0.5, -0.5), 0x0000FFFF);
}

#[test]
fn test_sample_nearest_center_of_texel() {
    let mut tex = Texture::new(4, 4);
    tex.set_pixel(2, 2, 0xFFFF00FF); // Yellow at (2, 2)

    // Center of texel (2, 2) in UV space
    // texel 2 covers [0.5, 0.75) in UV space for a 4x4 texture
    let u = (2.0 + 0.5) / 4.0; // 0.625
    let v = (2.0 + 0.5) / 4.0;
    assert_eq!(tex.sample_nearest(u, v), 0xFFFF00FF);
}

// TASK 4: Bilinear sampling
#[test]
fn test_sample_bilinear_exact_texel() {
    let mut tex = Texture::new(4, 4);
    tex.set_pixel(1, 1, 0xFF0000FF); // Red

    // Sample exactly at texel center - should return pure color
    let u = (1.0 + 0.5) / 4.0;
    let v = (1.0 + 0.5) / 4.0;
    assert_eq!(tex.sample_bilinear(u, v), 0xFF0000FF);
}

#[test]
fn test_sample_bilinear_interpolation() {
    let mut tex = Texture::new(2, 2);
    // Create a 2x2 texture with known values
    tex.set_pixel(0, 0, 0xFF000000); // Black (top-left)
    tex.set_pixel(1, 0, 0xFFFFFFFF); // White (top-right)
    tex.set_pixel(0, 1, 0xFF000000); // Black (bottom-left)
    tex.set_pixel(1, 1, 0xFFFFFFFF); // White (bottom-right)

    // Sample at center should blend all 4 texels
    // Midpoint between black and white should be gray
    let color = tex.sample_bilinear(0.5, 0.5);

    // Extract RGB channels (ARGB format: 0xAARRGGBB)
    let r = ((color >> 16) & 0xFF) as u8;
    let g = ((color >> 8) & 0xFF) as u8;
    let b = (color & 0xFF) as u8;

    // Should be approximately 50% gray (127-128 for each channel)
    assert!(
        (r as i32 - 127).abs() <= 1,
        "Red channel should be ~127, got {}",
        r
    );
    assert!(
        (g as i32 - 127).abs() <= 1,
        "Green channel should be ~127, got {}",
        g
    );
    assert!(
        (b as i32 - 127).abs() <= 1,
        "Blue channel should be ~127, got {}",
        b
    );
}

#[test]
fn test_sample_bilinear_wrapping() {
    let mut tex = Texture::new(2, 2);
    tex.set_pixel(1, 1, 0x00FF00FF); // Green

    // UV > 1.0 should wrap and still interpolate correctly
    let color = tex.sample_bilinear(1.75, 1.75);
    // At (0.75, 0.75) with green at (1, 1), should interpolate
    // The exact color depends on neighboring texels (black by default)
    assert_ne!(color, 0xFF000000); // Should not be pure black
}

// TASK 5: BMP File Loading
#[test]
fn test_bmp_load_invalid_magic() {
    let data = vec![0x00, 0x00]; // Not "BM"
    let result = Texture::from_bmp(&data);
    assert!(result.is_err());
}

#[test]
fn test_bmp_load_too_small() {
    let data = vec![b'B', b'M']; // Only magic bytes
    let result = Texture::from_bmp(&data);
    assert!(result.is_err());
}

#[test]
fn test_bmp_load_unsupported_bpp() {
    // Create minimal BMP header with unsupported bits per pixel (16-bit)
    let mut data = create_minimal_bmp_header(2, 2, 16);
    let result = Texture::from_bmp(&data);
    assert!(result.is_err());
}

// Helper function to create minimal BMP file data
fn create_minimal_bmp_header(width: u32, height: u32, bpp: u16) -> Vec<u8> {
    let mut data = Vec::new();

    // BMP File Header (14 bytes)
    data.extend_from_slice(b"BM"); // Magic
    data.extend_from_slice(&0u32.to_le_bytes()); // File size (placeholder)
    data.extend_from_slice(&0u32.to_le_bytes()); // Reserved
    data.extend_from_slice(&54u32.to_le_bytes()); // Data offset

    // DIB Header (40 bytes - BITMAPINFOHEADER)
    data.extend_from_slice(&40u32.to_le_bytes()); // Header size
    data.extend_from_slice(&(width as i32).to_le_bytes()); // Width
    data.extend_from_slice(&(height as i32).to_le_bytes()); // Height
    data.extend_from_slice(&1u16.to_le_bytes()); // Planes
    data.extend_from_slice(&bpp.to_le_bytes()); // Bits per pixel
    data.extend_from_slice(&0u32.to_le_bytes()); // Compression
    data.extend_from_slice(&0u32.to_le_bytes()); // Image size
    data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
    data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
    data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
    data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

    data
}

#[test]
fn test_bmp_load_24bit_2x2() {
    // Create a valid 2x2 24-bit BMP
    let mut data = create_minimal_bmp_header(2, 2, 24);

    // Pixel data (BGR format, bottom-up, row padding to 4-byte boundary)
    // 2x2 @ 24bpp = 6 bytes per row, padded to 8 bytes
    // Row 0 (bottom): Blue, Red + 2 padding bytes
    data.extend_from_slice(&[0xFF, 0x00, 0x00]); // Blue (BGR)
    data.extend_from_slice(&[0x00, 0x00, 0xFF]); // Red (BGR)
    data.extend_from_slice(&[0x00, 0x00]); // Padding

    // Row 1 (top): Green, Yellow + 2 padding bytes
    data.extend_from_slice(&[0x00, 0xFF, 0x00]); // Green (BGR)
    data.extend_from_slice(&[0x00, 0xFF, 0xFF]); // Yellow (BGR)
    data.extend_from_slice(&[0x00, 0x00]); // Padding

    let tex = Texture::from_bmp(&data).expect("Failed to load BMP");
    assert_eq!(tex.width(), 2);
    assert_eq!(tex.height(), 2);

    // Verify colors (remember BMP is bottom-up, so row 0 becomes row 1)
    // Top-left should be Green
    let top_left = tex.get_pixel(0, 0);
    assert_eq!(top_left, 0xFF00FF00); // ARGB: Green

    // Top-right should be Yellow
    let top_right = tex.get_pixel(1, 0);
    assert_eq!(top_right, 0xFFFFFF00); // ARGB: Yellow

    // Bottom-left should be Blue
    let bottom_left = tex.get_pixel(0, 1);
    assert_eq!(bottom_left, 0xFF0000FF); // ARGB: Blue

    // Bottom-right should be Red
    let bottom_right = tex.get_pixel(1, 1);
    assert_eq!(bottom_right, 0xFFFF0000); // ARGB: Red
}

// TASK 7: Perspective-correct UV interpolation
#[test]
fn test_perspective_interpolation_differs_from_linear() {
    let uv0 = Vec2::new(0.0, 0.0);
    let uv1 = Vec2::new(1.0, 1.0);
    let w0 = 1.0;
    let w1 = 2.0; // Different w values

    // Interpolate at t=0.5
    let t = 0.5;
    let perspective_uv = interpolate_uv_perspective(uv0, w0, uv1, w1, t);

    // Linear interpolation would give (0.5, 0.5)
    let linear_uv = Vec2::lerp(uv0, uv1, t);

    // With different w values, perspective should differ from linear
    assert!(
        (perspective_uv.x - linear_uv.x).abs() > 0.01
            || (perspective_uv.y - linear_uv.y).abs() > 0.01,
        "Perspective interpolation should differ from linear when w values differ"
    );
}

#[test]
fn test_perspective_interpolation_same_w_equals_linear() {
    let uv0 = Vec2::new(0.2, 0.3);
    let uv1 = Vec2::new(0.8, 0.9);
    let w = 1.5; // Same w for both

    let perspective_uv = interpolate_uv_perspective(uv0, w, uv1, w, 0.5);
    let linear_uv = Vec2::lerp(uv0, uv1, 0.5);

    // When w values are equal, perspective should match linear
    assert!((perspective_uv.x - linear_uv.x).abs() < 0.001);
    assert!((perspective_uv.y - linear_uv.y).abs() < 0.001);
}

#[test]
fn test_perspective_interpolation_endpoints() {
    let uv0 = Vec2::new(0.1, 0.2);
    let uv1 = Vec2::new(0.7, 0.8);
    let w0 = 1.0;
    let w1 = 3.0;

    // At t=0, should return uv0
    let result = interpolate_uv_perspective(uv0, w0, uv1, w1, 0.0);
    assert!((result.x - uv0.x).abs() < 0.001);
    assert!((result.y - uv0.y).abs() < 0.001);

    // At t=1, should return uv1
    let result = interpolate_uv_perspective(uv0, w0, uv1, w1, 1.0);
    assert!((result.x - uv1.x).abs() < 0.001);
    assert!((result.y - uv1.y).abs() < 0.001);
}
