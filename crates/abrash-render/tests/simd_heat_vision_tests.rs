use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::heat_vision::apply_heat_vision;

#[test]
fn test_heat_vision_simd_red_phase() {
    let width = 16;
    let height = 1;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    zb.test_and_set(0, 0, 0.0); // t = 0 (Red)
    zb.test_and_set(1, 0, 2.5); // t = 256 (Yellow)
    zb.test_and_set(2, 0, 5.0); // t = 512 (Green)
    zb.test_and_set(3, 0, 7.5); // t = 768 (Cyan)
    zb.test_and_set(4, 0, 10.0); // t = 1023 (Blue)

    apply_heat_vision(&mut fb, &zb);

    assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFF_0000, "t=0 should be Red");
    assert_eq!(fb.get_pixel(1, 0).unwrap(), 0xFFFF_FF00, "t=256 should be Yellow (Green component maxed)");
    assert_eq!(fb.get_pixel(2, 0).unwrap(), 0xFF00_FF00, "t=512 should be Green");
    assert_eq!(fb.get_pixel(3, 0).unwrap(), 0xFF00_FFFF, "t=768 should be Cyan (Blue component maxed)");
    assert_eq!(fb.get_pixel(4, 0).unwrap(), 0xFF00_00FF, "t=1023 should be Blue");
}
