#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::synthwave::apply_synthwave;

#[test]
fn test_synthwave_basic() {
    let mut fb = Framebuffer::new(10, 10).unwrap();
    let mut zb = ZBuffer::new(10, 10).unwrap();

    for y in 0..10 {
        for x in 0..10 {
            zb.test_and_set(x, y, (x + y) as f32);
        }
    }

    apply_synthwave(&mut fb, &zb);

    // After applying synthwave, the pixel should be modified
    assert_ne!(fb.get_pixel(0, 0).unwrap(), 0);
}
