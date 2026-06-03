use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::pbr::fill_triangle_pbr;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_pbr_rasterization() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Triangle vertices: ((Pos, W), Normal, WorldPos)
    let v0 = (
        (Vec3::new(0.0, 50.0, 0.0), 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let v1 = (
        (Vec3::new(-50.0, -50.0, 0.0), 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(-1.0, -1.0, 0.0),
    );
    let v2 = (
        (Vec3::new(50.0, -50.0, 0.0), 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, -1.0, 0.0),
    );

    let albedo = Vec3::new(1.0, 0.0, 0.0); // Red
    let metallic = 0.0;
    let roughness = 0.5;
    let ao = 1.0;

    let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let view_pos = Vec3::new(0.0, 0.0, 5.0);

    fill_triangle_pbr(
        &mut fb,
        &mut zb,
        v0,
        v1,
        v2,
        albedo,
        metallic,
        roughness,
        ao,
        light_dir,
        light_color,
        view_pos,
    );

    // Check if something was drawn (center pixel should be red-ish)
    let center_pixel = fb.get_pixel(50, 50).unwrap();
    assert_ne!(center_pixel, 0xFF00_0000); // Not black
}
#[test]
fn test_pbr_rasterization_extreme_brightness() {
    let width = 10;
    let height = 10;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let v0 = (
        (Vec3::new(0.0, 5.0, 0.0), 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let v1 = (
        (Vec3::new(-5.0, -5.0, 0.0), 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(-1.0, -1.0, 0.0),
    );
    let v2 = (
        (Vec3::new(5.0, -5.0, 0.0), 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, -1.0, 0.0),
    );

    let albedo = Vec3::new(1.0, 1.0, 1.0);
    let metallic = 1.0;
    let roughness = 0.0;
    let ao = 1.0;

    let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
    let light_color = Vec3::new(1000.0, 1000.0, 1000.0);
    let view_pos = Vec3::new(0.0, 0.0, 5.0);

    fill_triangle_pbr(
        &mut fb,
        &mut zb,
        v0,
        v1,
        v2,
        albedo,
        metallic,
        roughness,
        ao,
        light_dir,
        light_color,
        view_pos,
    );

    let center_pixel = fb.get_pixel(5, 5).unwrap();
    let r = (center_pixel >> 16) & 0xFF;
    let g = (center_pixel >> 8) & 0xFF;
    let b = center_pixel & 0xFF;

    assert!(r > 40, "Red channel should be bright, got {r}");
    assert!(g > 40, "Green channel should be bright, got {g}");
    assert!(b > 40, "Blue channel should be bright, got {b}");
}
