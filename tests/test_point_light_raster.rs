use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_point_lit;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_point_light_rasterization_runs() {
    let width = 64;
    let height = 64;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let w0 = Vec3::new(0.0, 9.0, 0.0);
    let w1 = Vec3::new(-9.0, -9.0, 0.0);
    let w2 = Vec3::new(9.0, -9.0, 0.0);

    let v0 = (
        (Vec3::new(0.0, 0.9, 5.0), 5.0),
        Vec3::new(0.0, 0.0, 1.0),
        w0,
    );
    let v1 = (
        (Vec3::new(-0.9, -0.9, 5.0), 5.0),
        Vec3::new(0.0, 0.0, 1.0),
        w1,
    );
    let v2 = (
        (Vec3::new(0.9, -0.9, 5.0), 5.0),
        Vec3::new(0.0, 0.0, 1.0),
        w2,
    );

    let light_pos = Vec3::new(0.0, 0.0, 5.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let attenuation = Vec3::new(0.0, 0.1, 0.01);
    let color = Vec3::new(1.0, 1.0, 1.0);

    fb.clear(0);
    zb.clear();

    fill_triangle_point_lit(
        &mut fb,
        &mut zb,
        v0,
        v1,
        v2,
        color,
        light_pos,
        light_color,
        attenuation,
    );

    // Ensure we rendered *something* other than black background
    let mut drew_pixels = false;
    for &pixel in fb.as_slice() {
        if pixel != 0 {
            drew_pixels = true;
            break;
        }
    }
    assert!(drew_pixels, "Point light rasterization drew nothing!");
}
