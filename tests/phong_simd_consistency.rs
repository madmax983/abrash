use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::rasterizer::phong::{fill_triangle_phong_shadowed, fill_triangle_point_lit};
use abrash::zbuffer::ZBuffer;

#[test]
fn test_phong_simd_consistency() {
    let width = 100;
    let height = 100;
    let mut fb_simd = Framebuffer::new(width as u32, height as u32).unwrap();
    let mut zb_simd = ZBuffer::new(width as u32, height as u32).unwrap();

    let mut fb_shadow = Framebuffer::new(width as u32, height as u32).unwrap();
    let mut zb_shadow = ZBuffer::new(width as u32, height as u32).unwrap();

    // Large triangle covering screen
    let v0 = (
        (Vec3::new(-100.0, -100.0, 5.0), 5.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(-10.0, -10.0, 5.0),
    );
    let v1 = (
        (Vec3::new(100.0, -100.0, 5.0), 5.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(10.0, -10.0, 5.0),
    );
    let v2 = (
        (Vec3::new(0.0, 100.0, 5.0), 5.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 10.0, 5.0),
    );

    let color = Vec3::new(1.0, 1.0, 1.0);
    let light_pos = Vec3::new(0.0, 0.0, 0.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let attenuation = Vec3::new(1.0, 0.0, 0.0);

    fill_triangle_point_lit(
        &mut fb_simd,
        &mut zb_simd,
        v0,
        v1,
        v2,
        color,
        light_pos,
        light_color,
        attenuation,
    );

    let mut pixel_count = 0;
    for y in 0..height {
        for x in 0..width {
            let p = fb_simd.get_pixel(x, y).unwrap();
            if p != 0x00000000 && p != 0xFF000000 {
                pixel_count += 1;
            }
        }
    }
    assert!(
        pixel_count > 0,
        "SIMD point lit rasterizer should render some pixels"
    );

    fill_triangle_phong_shadowed(
        &mut fb_shadow,
        &mut zb_shadow,
        v0,
        v1,
        v2,
        color,
        Vec3::new(0.0, 0.0, -1.0),
        light_color,
        Vec3::new(0.1, 0.1, 0.1),
        &zb_simd,
        Mat4::identity(),
    );

    let mut pixel_count_shadow = 0;
    for y in 0..height {
        for x in 0..width {
            let p = fb_shadow.get_pixel(x, y).unwrap();
            if p != 0x00000000 && p != 0xFF000000 {
                pixel_count_shadow += 1;
            }
        }
    }
    assert!(
        pixel_count_shadow > 0,
        "SIMD shadow mapped rasterizer should render some pixels"
    );
}
