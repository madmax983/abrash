use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::rasterizer::line::draw_line_3d;
use proptest::prelude::*;

proptest! {
    #[test]
    fn havoc_line_3d_overflow(
        v0_x in any::<f32>(), v0_y in any::<f32>(), v0_z in any::<f32>(), v0_w in any::<f32>(),
        v1_x in any::<f32>(), v1_y in any::<f32>(), v1_z in any::<f32>(), v1_w in any::<f32>(),
    ) {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let mut zb = ZBuffer::new(100, 100).unwrap();

        let v0 = (Vec3::new(v0_x, v0_y, v0_z), v0_w);
        let v1 = (Vec3::new(v1_x, v1_y, v1_z), v1_w);

        draw_line_3d(&mut fb, &mut zb, v0, v1, 0xFFFF_FFFF);
    }
}
