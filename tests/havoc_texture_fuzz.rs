use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;
use proptest::prelude::*;

proptest! {
    // Run enough cases to hit edge cases
    #![proptest_config(ProptestConfig::with_cases(5000))]

    #[test]
    fn fuzz_texture_safety(
        // Texture dimensions: Small to hit edge cases
        tex_w in 1u32..=16u32,
        tex_h in 1u32..=16u32,
        // UVs: Can be large, negative, NaN
        u0 in any::<f32>(), v0 in any::<f32>(),
        u1 in any::<f32>(), v1 in any::<f32>(),
        u2 in any::<f32>(), v2 in any::<f32>(),
        // Vertices
        p0_x in -10.0f32..10.0f32, p0_y in -10.0f32..10.0f32, p0_z in 1.0f32..10.0f32,
        p1_x in -10.0f32..10.0f32, p1_y in -10.0f32..10.0f32, p1_z in 1.0f32..10.0f32,
        p2_x in -10.0f32..10.0f32, p2_y in -10.0f32..10.0f32, p2_z in 1.0f32..10.0f32,
        // Mode
        bilinear in any::<bool>(),
    ) {
        let width = 32;
        let height = 32;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        let mut tex = Texture::new(tex_w, tex_h).unwrap();
        // Fill texture with data
        for y in 0..tex_h {
            for x in 0..tex_w {
                tex.set_pixel(x, y, 0xFFFFFFFF);
            }
        }
        tex.filter_mode = if bilinear { FilterMode::Bilinear } else { FilterMode::Nearest };

        // Vertices in Clip Space (w=1.0 for simplicity, z used as w)
        // We use p.z as w to simulate perspective
        let v0 = ((Vec3::new(p0_x, p0_y, p0_z), p0_z), Vec2::new(u0, v0));
        let v1 = ((Vec3::new(p1_x, p1_y, p1_z), p1_z), Vec2::new(u1, v1));
        let v2 = ((Vec3::new(p2_x, p2_y, p2_z), p2_z), Vec2::new(u2, v2));

        fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &tex);
    }
}
