use abrash::texture::Texture;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[test]
fn test_load_ppm_p3() {
    let filename = "test_texture_p3.ppm";
    let ppm_content = "P3
2 2
255
255 0 0   0 255 0
0 0 255   255 255 255
";

    let mut file = File::create(filename).expect("Failed to create file");
    file.write_all(ppm_content.as_bytes()).expect("Failed to write to file");

    // Load texture
    let result = Texture::load_ppm(Path::new(filename));

    // Cleanup
    let _ = std::fs::remove_file(filename);

    let tex = result.expect("Failed to load PPM");

    assert_eq!(tex.width, 2);
    assert_eq!(tex.height, 2);

    // Verify pixels
    // (0,0) -> Red
    assert_eq!(tex.get_pixel_texel(0, 0), 0xFFFF0000);
    // (1,0) -> Green
    assert_eq!(tex.get_pixel_texel(1, 0), 0xFF00FF00);
    // (0,1) -> Blue
    assert_eq!(tex.get_pixel_texel(0, 1), 0xFF0000FF);
    // (1,1) -> White
    assert_eq!(tex.get_pixel_texel(1, 1), 0xFFFFFFFF);
}
