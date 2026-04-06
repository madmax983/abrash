use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::rasterizer::line::draw_line_3d;

#[test]
#[should_panic(expected = "Framebuffer and ZBuffer widths must match")]
fn test_draw_line_out_of_bounds() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(10, 10).unwrap();
    let v0 = (Vec3::new(0.8, 0.8, 0.5), 1.0);
    let v1 = (Vec3::new(0.8, 0.8, 0.5), 1.0);
    draw_line_3d(&mut fb, &mut zb, v0, v1, 0xFFFF_FFFF);
}
