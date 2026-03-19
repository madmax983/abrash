#[cfg(test)]
mod tests {
    use abrash::framebuffer::Framebuffer;
    use abrash::math::{Mat4, Vec3};
    use abrash::rasterizer::{fill_triangle_3d, fill_triangle_phong_shadowed};
    use abrash::zbuffer::ZBuffer;
    use std::f32::consts::PI;

    #[test]
    fn test_shadow_mapping_scene() {
        let width = 256;
        let height = 256;

        // 1. Setup Shadow Map (Light View)
        let light_pos = Vec3::new(0.0, 10.0, 0.1); // Slightly offset Z to define Up
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 0.0, 1.0); // Z-up

        let light_view = Mat4::look_at(light_pos, target, up);
        let light_proj = Mat4::perspective(PI / 2.0, 1.0, 1.0, 20.0);
        let light_vp = light_view * light_proj;

        let mut shadow_fb = Framebuffer::new(width, height).unwrap(); // Dummy FB
        let mut shadow_zb = ZBuffer::new(width, height).unwrap();

        // Occluder: Triangle at Y=5
        // Centered at (0, 5, 0), size 2.0
        let v0_world = Vec3::new(-1.0, 5.0, 0.0);
        let v1_world = Vec3::new(1.0, 5.0, 0.0);
        let v2_world = Vec3::new(0.0, 5.0, 1.0); // Triangle in XZ plane (tilted)

        // Transform to Light Clip Space
        let (v0_clip, w0) = light_vp.transform_point(v0_world);
        let (v1_clip, w1) = light_vp.transform_point(v1_world);
        let (v2_clip, w2) = light_vp.transform_point(v2_world);

        // Render to Shadow Map
        // We use fill_triangle_3d (flat) because we only care about depth
        fill_triangle_3d(
            &mut shadow_fb,
            &mut shadow_zb,
            (v0_clip, w0),
            (v2_clip, w2), // Swap v1 and v2 to reverse winding
            (v1_clip, w1),
            0xFFFF_FFFF,
        );

        // Check if shadow map has content at center
        let sm_center = shadow_fb.get_pixel(128, 128);
        println!("Shadow Map Center Pixel: {sm_center:?}");
        let sm_depth = shadow_zb.get_depth(128, 128);
        println!("Shadow Map Center Depth: {sm_depth:?}");

        // 2. Setup Main Camera
        let cam_pos = Vec3::new(0.0, 10.0, 10.0);
        let cam_target = Vec3::new(0.0, 0.0, 0.0);
        let cam_up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(cam_pos, cam_target, cam_up);
        let proj = Mat4::perspective(PI / 3.0, 1.0, 1.0, 50.0);
        let cam_vp = view * proj;

        let mut main_fb = Framebuffer::new(width, height).unwrap();
        let mut main_zb = ZBuffer::new(width, height).unwrap();
        main_fb.clear(0xFFFF_0000); // Red background

        // Receiver: Plane at Y=0
        // -5 to 5
        let p0 = Vec3::new(-5.0, 0.0, -5.0);
        let p1 = Vec3::new(5.0, 0.0, -5.0);
        let p2 = Vec3::new(-5.0, 0.0, 5.0);
        let p3 = Vec3::new(5.0, 0.0, 5.0);

        // Two triangles for the plane
        let plane_tris = [
            (p0, p2, p1), // Reversed winding
            (p1, p2, p3), // Reversed winding
        ];

        let light_dir = (target - light_pos).normalize();
        let light_color = Vec3::new(1.0, 1.0, 1.0);
        let ambient = Vec3::new(0.1, 0.1, 0.1);
        let material_color = Vec3::new(0.8, 0.8, 0.8);

        for (a, b, c) in plane_tris {
            // Transform to Camera Clip Space
            let (ac, aw) = cam_vp.transform_point(a);
            let (bc, bw) = cam_vp.transform_point(b);
            let (cc, cw) = cam_vp.transform_point(c);

            // Normals (Up, Y=1)
            let n = Vec3::new(0.0, 1.0, 0.0);

            // Render using Phong with Shadows
            fill_triangle_phong_shadowed(
                &mut main_fb,
                &mut main_zb,
                ((ac, aw), n, a), // Pass World Pos
                ((bc, bw), n, b),
                ((cc, cw), n, c),
                material_color,
                light_dir,
                light_color,
                ambient,
                &shadow_zb,
                light_vp,
            );
        }

        // 3. Verify
        // Sample a point we expect to be in shadow (directly under the occluder).
        // Occluder is at (0, 5, 0). Light is at (0, 10, 0).
        // Shadow should be around (0, 0, 0).

        // Sample a point we expect to be lit (far away).
        // (4, 0, 4)

        // Since we currently use standard Phong, both should be lit.
        // Once we switch to Shadowed Phong, center should be dark.

        // Convert world points to screen to find pixel coords
        let (center_clip, center_w) = cam_vp.transform_point(Vec3::new(0.0, 0.0, 0.0));
        let center_screen = abrash::math::project_to_screen(center_clip, center_w, width, height);

        let (far_clip, far_w) = cam_vp.transform_point(Vec3::new(3.0, 0.0, 3.0));
        let far_screen = abrash::math::project_to_screen(far_clip, far_w, width, height);

        println!("Center Screen: {center_screen:?}");
        println!("Far Screen: {far_screen:?}");

        let center_pixel = main_fb
            .get_pixel(center_screen.x, center_screen.y)
            .expect("Center pixel out of bounds");
        let far_pixel = main_fb
            .get_pixel(far_screen.x, far_screen.y)
            .expect("Far pixel out of bounds");

        let center_brightness =
            (center_pixel & 0xFF) + ((center_pixel >> 8) & 0xFF) + ((center_pixel >> 16) & 0xFF);
        let far_brightness =
            (far_pixel & 0xFF) + ((far_pixel >> 8) & 0xFF) + ((far_pixel >> 16) & 0xFF);

        println!("Center Brightness: {center_brightness}");
        println!("Far Brightness: {far_brightness}");

        // Expectation:
        // With standard Phong: Center ~= Far (roughly, based on angle)
        // With Shadowed Phong: Center << Far

        // Assert failure to represent "Red" phase
        // We simulate the failure of "Shadow test"
        // In a real TDD step, we would call the function and it would fail to compile or run.
        // Here, we assert that the shadow effect is MISSING.

        // If we want to strictly follow "Red" as "Test Fails", we should assert that
        // center is significantly darker than far, which will FAIL now.

        assert!(
            center_brightness < far_brightness / 2,
            "Center should be in shadow (darker than far)"
        );
    }
}
