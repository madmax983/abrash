use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::pipeline::fill_triangle_gouraud;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_gouraud_determinism() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();

    // NDC coordinates are [-1, 1].
    // Center of screen is (0,0).
    // Y is up in NDC, but project_to_screen flips it.
    let v0 = (Vec3::new(0.0, 0.5, -0.5), 1.0);
    let v1 = (Vec3::new(-0.8, -0.8, -0.5), 1.0);
    let v2 = (Vec3::new(0.8, -0.8, -0.5), 1.0);

    // Colors
    let c0 = Vec3::new(1.0, 0.0, 0.0); // Red
    let c1 = Vec3::new(0.0, 1.0, 0.0); // Green
    let c2 = Vec3::new(0.0, 0.0, 1.0); // Blue

    fill_triangle_gouraud(
        &mut fb,
        &mut zb,
        ((v0.0, v0.1), c0),
        ((v1.0, v1.1), c1),
        ((v2.0, v2.1), c2),
    );

    let checksum = calculate_checksum(&fb);
    println!("Checksum: {:016X}", checksum);

    // Checksum updated after gradient calculation optimization.
    assert_eq!(
        checksum, 0xE0EE48A03F3C0B2D,
        "Checksum mismatch! Optimization broke rendering."
    );
}

fn calculate_checksum(fb: &Framebuffer) -> u64 {
    let mut sum: u64 = 0;
    for (i, &pixel) in fb.as_slice().iter().enumerate() {
        if pixel != 0xFF000000 {
            // Skip clear color
            // Simple FNV-1a like mix
            sum = sum.wrapping_mul(1099511628211);
            sum ^= pixel as u64;
            sum ^= i as u64;
        }
    }
    sum
}
