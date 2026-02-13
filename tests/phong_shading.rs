use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_phong;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_phong_shading_compiles_and_runs() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Triangle covering most of the screen
    // Vertices: ((Pos, W), Normal)
    let v0 = ((Vec3::new(0.0, 0.9, 5.0), 5.0), Vec3::new(0.0, 1.0, 0.0));
    let v1 = (
        (Vec3::new(-0.9, -0.9, 5.0), 5.0),
        Vec3::new(-0.7, -0.7, 0.7).normalize(),
    );
    let v2 = (
        (Vec3::new(0.9, -0.9, 5.0), 5.0),
        Vec3::new(0.7, -0.7, 0.7).normalize(),
    );

    let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize(); // Light from camera
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.1, 0.1, 0.1);
    let color = Vec3::new(1.0, 0.0, 0.0); // Red material
    let view_dir = Vec3::new(0.0, 0.0, 1.0);
    let specular_strength = 0.0;
    let shininess = 1.0;

    fill_triangle_phong(
        &mut fb,
        &mut zb,
        v0,
        v1,
        v2,
        color,
        light_dir,
        light_color,
        ambient,
        view_dir,
        specular_strength,
        shininess,
    );

    // Check center pixel
    let center_pixel = fb.get_pixel(50, 50).unwrap();
    assert_ne!(center_pixel, 0xFF00_0000, "Center pixel should be drawn");

    // Check that pixels have different colors (gradient)
    // Pixel near top (v0 has normal up, light is front, dot=0) -> darker?
    // Wait, light is (0,0,-1).
    // v0 normal is (0,1,0). Dot is 0. Diffuse is 0. Ambient only.
    // v1 normal is (-0.7, -0.7, 0.7). Dot with (0,0,-1) is -0.7. Clamped to 0. Ambient only.
    // v2 normal is (0.7, -0.7, 0.7). Dot with (0,0,-1) is -0.7. Clamped to 0.

    // Let's adjust light to be from top-front
    // light_dir = (0, -1, -1).normalize() -> points down-forward.

    // For this test, I just want to verify it runs.
    // I'll check that we have some non-black pixels.
    let drawn_pixels = fb.as_slice().iter().filter(|&&p| p != 0xFF00_0000).count();
    assert!(drawn_pixels > 100);
}

#[test]
fn test_phong_shading_gradient() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Triangle with different normals
    // v0: Normal points to light -> bright
    // v1: Normal points away -> dark
    let light_dir = Vec3::new(0.0, 0.0, -1.0); // Light comes from +Z (camera) towards -Z

    // Use w=1.0 so the triangle covers -0.9 to 0.9 in NDC (almost full screen)
    let v0 = ((Vec3::new(0.0, 0.9, 1.0), 1.0), Vec3::new(0.0, 0.0, 1.0)); // Points to +Z (light source) -> Bright
    let v1 = ((Vec3::new(-0.9, -0.9, 1.0), 1.0), Vec3::new(-1.0, 0.0, 0.0)); // Points Left -> Dark
    let v2 = ((Vec3::new(0.9, -0.9, 1.0), 1.0), Vec3::new(1.0, 0.0, 0.0)); // Points Right -> Dark

    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.0, 0.0, 0.0); // No ambient to ensure contrast
    let color = Vec3::new(1.0, 1.0, 1.0); // White material
    let view_dir = Vec3::new(0.0, 0.0, 1.0);
    let specular_strength = 0.0;
    let shininess = 1.0;

    fill_triangle_phong(
        &mut fb,
        &mut zb,
        v0,
        v1,
        v2,
        color,
        light_dir,
        light_color,
        ambient,
        view_dir,
        specular_strength,
        shininess,
    );

    // Top pixel (near v0) should be bright
    let top_pixel = fb.get_pixel(50, 10).unwrap();
    let bottom_left = fb.get_pixel(10, 90).unwrap();

    // extract brightness (G channel since white)
    let top_g = (top_pixel >> 8) & 0xFF;
    let bottom_g = (bottom_left >> 8) & 0xFF;

    assert!(
        top_g > bottom_g,
        "Top pixel should be brighter than bottom pixel ({top_g} vs {bottom_g})"
    );
}
