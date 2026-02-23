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
    assert_ne!(center_pixel, 0xFF000000); // Not black
}
