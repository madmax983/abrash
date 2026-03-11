use abrash::framebuffer::Framebuffer;
use abrash::math::Mat4;
use abrash::post_process;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

#[test]
fn test_apply_ssao_darkens_occluded_pixels() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Fill FB with white
    fb.clear(0xFFFFFFFF);

    // Setup Z-buffer with a "corner"
    // Let's try to simulate a corner.
    // We need the depth values to be realistic for the projection matrix.
    // Let's use a standard projection.
    let proj = Mat4::perspective(PI / 2.0, 1.0, 0.1, 100.0);

    // Clear ZB to 1.0 (far)
    zb.clear(); // Clears to INFINITY, but we want 1.0 for this test logic usually?
    // Wait, ZBuffer::clear sets to f32::MAX usually.
    // But for SSAO we usually assume far plane is 1.0 in NDC if we use standard depth range.
    // Let's check ZBuffer implementation.
    // ZBuffer clear uses f32::INFINITY.
    // Let's manually set background to 1.0 (far plane in NDC)
    for y in 0..height {
        for x in 0..width {
            zb.test_and_set(x as i32, y as i32, 1.0);
        }
    }

    // Draw a flat wall at z=-10.0 (view space).
    // z_ndc = -P22 - P32/z_view
    // P22 = (100.1 / -99.9) approx -1.002
    // P32 = (2*100*0.1 / -99.9) approx -0.2
    // z_ndc = 1.002 - (-0.2 / -10) = 1.002 - 0.02 = 0.982
    let wall_depth = 0.982;

    // Draw a "post" in front of it at z=-9.6 (close to wall for SSAO).
    // z_ndc = 1.002 - (-0.2 / -9.6) = 1.002 - 0.0208 = 0.9812
    let post_depth = 0.9812;

    for y in 0..height {
        for x in 0..width {
            zb.test_and_set(x as i32, y as i32, wall_depth);
        }
    }

    // Post in the center
    for y in 40..60 {
        for x in 40..60 {
            zb.test_and_set(x, y, post_depth);
        }
    }

    // Apply SSAO (radius 1.0 to cover gap 0.4)
    post_process::apply_ssao(&mut fb, &zb, &proj, &post_process::SsaoConfig { radius: 1.0, bias: 0.001, intensity: 2.0 });

    // Check pixels near the post (e.g., 39, 50).
    // They should be darkened because the post occludes the wall.
    // The post is at 40..60. Pixel 39 is just outside.
    // Samples from 39 will hit the post (depth 0.962) which is closer than wall (0.982).
    let p_occluded = fb.get_pixel(39, 50).unwrap();
    let p_unoccluded = fb.get_pixel(10, 50).unwrap();

    let lum_occluded = p_occluded & 0xFF;
    let lum_unoccluded = p_unoccluded & 0xFF;

    assert!(
        lum_occluded < lum_unoccluded,
        "Pixel near post should be darker (got {lum_occluded} vs {lum_unoccluded})"
    );

    // Also ensure it didn't turn black (sanity check)
    assert!(lum_occluded > 0, "Pixel shouldn't be completely black");
}
