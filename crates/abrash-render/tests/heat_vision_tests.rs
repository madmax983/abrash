use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::heat_vision::apply_heat_vision;

#[test]
fn test_heat_vision_renders_correctly() {
    let mut fb = Framebuffer::new(5, 1).unwrap();
    let mut zb = ZBuffer::new(5, 1).unwrap();

    zb.test_and_set(0, 0, 1.0);
    zb.test_and_set(1, 0, 2.0);
    zb.test_and_set(2, 0, 3.0);
    zb.test_and_set(3, 0, 4.0);
    zb.test_and_set(4, 0, 5.0);

    apply_heat_vision(&mut fb, &zb);

    assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFF_0000);
    assert_eq!(fb.get_pixel(4, 0).unwrap(), 0xFF00_00FF);
}
