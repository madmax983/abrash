use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::texture::Texture;
use abrash::math::{Vec3, Vec2};
use abrash::rasterizer::fill_triangle_textured;
use proptest::prelude::*;

prop_compose! {
    fn arb_vec3()(x in any::<f32>(), y in any::<f32>(), z in any::<f32>()) -> Vec3 {
        Vec3::new(x, y, z)
    }
}

prop_compose! {
    fn arb_vec2()(x in any::<f32>(), y in any::<f32>()) -> Vec2 {
        Vec2::new(x, y)
    }
}

prop_compose! {
    fn arb_dims()(w in 1u32..256, h in 1u32..256) -> (u32, u32) {
        (w, h)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn test_fill_triangle_textured_havoc(
        (w, h) in arb_dims(),
        (tex_w, tex_h) in arb_dims(),
        v0_pos in arb_vec3(), v0_w in any::<f32>(), v0_uv in arb_vec2(),
        v1_pos in arb_vec3(), v1_w in any::<f32>(), v1_uv in arb_vec2(),
        v2_pos in arb_vec3(), v2_w in any::<f32>(), v2_uv in arb_vec2(),
    ) {
        let mut fb = Framebuffer::new(w, h).unwrap();
        let mut zb = ZBuffer::new(w, h).unwrap();

        // Texture::checkered is easier.
        let texture = Texture::checkered(tex_w, tex_h, 0xFFFFFFFF, 0xFF000000).unwrap();

        let v0 = ((v0_pos, v0_w), v0_uv);
        let v1 = ((v1_pos, v1_w), v1_uv);
        let v2 = ((v2_pos, v2_w), v2_uv);

        fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &texture);
    }
}
