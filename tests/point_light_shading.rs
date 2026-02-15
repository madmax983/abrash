use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_point_lit;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_point_light_attenuation() {
    let width = 200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Triangle coordinates
    // We want to sample at Y=60 (Screen).
    // Screen Height = 100. Half = 50.
    // screen_y = (1 - ndc_y) * 50.
    // 60 = (1 - y) * 50 => 1.2 = 1 - y => y = -0.2.

    // So if we set the bottom vertices at y = -0.2, they will land exactly at Y=60.

    let z_depth = 0.5; // Inside frustum

    let p0_clip = (Vec3::new(-0.9, -0.2, z_depth), 1.0);
    let p1_clip = (Vec3::new(0.9, -0.2, z_depth), 1.0);
    let p2_clip = (Vec3::new(0.0, 0.5, z_depth), 1.0);

    let w0 = Vec3::new(-10.0, 0.0, 0.0);
    let w1 = Vec3::new(10.0, 0.0, 0.0);
    let w2 = Vec3::new(0.0, 5.0, 0.0);

    let normal = Vec3::new(0.0, 0.0, 1.0); // Facing camera

    let v0 = (p0_clip, normal, w0);
    let v1 = (p1_clip, normal, w1);
    let v2 = (p2_clip, normal, w2);

    let color = Vec3::new(1.0, 1.0, 1.0); // White surface

    // Light Position: Far left (-15, 0, 2)
    // Light is closer to v0 (-10) than v1 (10).
    let light_pos = Vec3::new(-15.0, 0.0, 2.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);

    // Attenuation: 1.0 / (0.0 + 0.1 * d + 0.01 * d^2)
    let attenuation = Vec3::new(0.0, 0.1, 0.01);

    fill_triangle_point_lit(
        &mut fb, &mut zb,
        v0, v1, v2,
        color,
        light_pos,
        light_color,
        attenuation
    );

    // Sample pixels along the horizontal line y=60 (Clip Y=-0.2)
    // At this Y, the triangle width is maximal (-0.9 to 0.9)
    // Screen X for -0.9: (-0.9 + 1) * 100 = 10.
    // Screen X for 0.9: (0.9 + 1) * 100 = 190.

    let x_left = 20;
    let x_right = 180;
    let y_sample = 60;

    let pixel_left = fb.get_pixel(x_left, y_sample).expect("Pixel left should be drawn");
    let pixel_right = fb.get_pixel(x_right, y_sample).expect("Pixel right should be drawn");

    let brightness_left = (pixel_left & 0xFF) as i32;
    let brightness_right = (pixel_right & 0xFF) as i32;

    println!("Left brightness (at {},{}): {}", x_left, y_sample, brightness_left);
    println!("Right brightness (at {},{}): {}", x_right, y_sample, brightness_right);

    // Also check center
    let pixel_center = fb.get_pixel(100, y_sample).expect("Pixel center should be drawn");
    let brightness_center = (pixel_center & 0xFF) as i32;
    println!("Center brightness (at 100,{}): {}", y_sample, brightness_center);

    assert!(brightness_left > 0, "Left side should be lit");
    assert!(brightness_right > 0, "Right side should be lit");
    assert!(brightness_left > brightness_right, "Left side (closer to light) should be brighter");
}
