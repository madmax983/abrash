use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::math::Vec3;
use abrash::rasterizer::pbr::fill_triangle_pbr;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn test_pbr_simd_fuzz(
        x0 in -100.0f32..100.0, y0 in -100.0f32..100.0, z0 in 0.1f32..100.0,
        x1 in -100.0f32..100.0, y1 in -100.0f32..100.0, z1 in 0.1f32..100.0,
        x2 in -100.0f32..100.0, y2 in -100.0f32..100.0, z2 in 0.1f32..100.0,
    ) {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        let mut zb = ZBuffer::new(32, 32).unwrap();

        let v0 = ((Vec3::new(x0, y0, z0), 1.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 0.0));
        let v1 = ((Vec3::new(x1, y1, z1), 1.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 0.0));
        let v2 = ((Vec3::new(x2, y2, z2), 1.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 0.0));

        let albedo = Vec3::new(1.0, 1.0, 1.0);
        let metallic = 1.0;
        let roughness = 1.0;
        let ao = 1.0;
        let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
        let light_color = Vec3::new(1.0, 1.0, 1.0);
        let view_pos = Vec3::new(0.0, 0.0, 0.0);

        fill_triangle_pbr(
            &mut fb, &mut zb,
            v0, v1, v2,
            albedo, metallic, roughness, ao,
            light_dir, light_color, view_pos
        );
    }
}
