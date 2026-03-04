use abrash::math::{Vec3, project_triangle_to_screen};

#[test]
fn test_havoc_resolution_no_overflow() {
    let width = 65536;
    let height = 65536;
    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    let v0 = (Vec3::new(0.831, -0.831, 1.0), 1.0);
    let v1 = (Vec3::new(0.9, -0.831, 1.0), 1.0);
    let v2 = (Vec3::new(0.831, -0.9, 1.0), 1.0);

    let (_p0, p1, p2) =
        project_triangle_to_screen(v0.0, v0.1, v1.0, v1.1, v2.0, v2.1, half_width, half_height);

    assert!(p1.x > 32767, "Projection is not over i16::MAX");
    assert!(p2.y > 32767, "Projection is not over i16::MAX");

    let tri_x_i32 = p1.x; // Previously `p1.x as i16` which caused overflow
    let tri_y_i32 = p2.y; // Previously `p2.y as i16` which caused overflow

    assert!(tri_x_i32 > 0, "Overflow reproduced");
    assert!(tri_y_i32 > 0, "Overflow reproduced");
}
