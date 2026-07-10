use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::rasterizer::gouraud::{fill_triangle_gouraud};
use abrash_core::math::Vec3;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]
    #[test]
    #[ignore = "👹 Havoc: Trigger attempt to add with overflow panic"]
    fn havoc_gouraud_overflow(
        x1 in any::<f32>(), y1 in any::<f32>(), z1 in any::<f32>(), i1 in any::<f32>(), n1x in any::<f32>(), n1y in any::<f32>(), n1z in any::<f32>(),
        x2 in any::<f32>(), y2 in any::<f32>(), z2 in any::<f32>(), i2 in any::<f32>(), n2x in any::<f32>(), n2y in any::<f32>(), n2z in any::<f32>(),
        x3 in any::<f32>(), y3 in any::<f32>(), z3 in any::<f32>(), i3 in any::<f32>(), n3x in any::<f32>(), n3y in any::<f32>(), n3z in any::<f32>(),
        fb_w in 1u32..200u32, fb_h in 1u32..200u32,
    ) {
        let mut fb = Framebuffer::new(fb_w, fb_h).unwrap();
        let mut zb = ZBuffer::new(fb_w, fb_h).unwrap();

        let p1 = ((Vec3::new(x1, y1, z1), i1), Vec3::new(n1x, n1y, n1z));
        let p2 = ((Vec3::new(x2, y2, z2), i2), Vec3::new(n2x, n2y, n2z));
        let p3 = ((Vec3::new(x3, y3, z3), i3), Vec3::new(n3x, n3y, n3z));

        fill_triangle_gouraud(&mut fb, &mut zb, p1, p2, p3);
    }
}
