use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::math::Vec3;
use abrash::pipeline::fill_triangle_3d;

#[test]
fn scanline_overflow_check() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height);
    let mut zb = ZBuffer::new(width, height);

    // Create a triangle with one vertex extremely far to the left
    // This should result in screen x coordinate being i32::MIN
    // triggering the (-xs) panic if not fixed.
    let v0 = (Vec3::new(-1e10, 0.0, 5.0), 1.0);
    let v1 = (Vec3::new(0.0, 10.0, 5.0), 1.0);
    let v2 = (Vec3::new(10.0, 0.0, 5.0), 1.0);

    // This panicked in debug mode before the fix due to negation of i32::MIN
    fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, 0xFFFFFFFF);
}

#[test]
#[should_panic]
fn framebuffer_overflow_check() {
    // Attempt to create a framebuffer that is way too big.
    // On 64-bit this will likely panic in vec! allocation (OOM or capacity overflow).
    // On 32-bit (if susceptible to wrap), our fix will ensure it panics with "overflow".
    // Either way, it should panic.
    let _fb = Framebuffer::new(u32::MAX, u32::MAX);
}
