//! Direct fuzzing and bounds testing for the Rasterizer using proptest.
use crate::rasterizer::texture::draw_span_nearest_simd;
use abrash_core::texture::Texture;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50_000))]
    #[test]
    #[ignore = "👹 Havoc: Direct Fuzzing of unsafe SIMD bounds mapping"]
    fn test_draw_span_nearest_simd_fuzz(
        z in any::<f32>(),
        dz_dx in any::<f32>(),
        u_fix in any::<i32>(),
        v_fix in any::<i32>(),
        du_fix in any::<i32>(),
        dv_fix in any::<i32>(),
        w in 1u32..2048u32,
        h in 1u32..2048u32,
        fb_len in 0usize..2048usize,
        zb_len in 0usize..2048usize
    ) {
        let mut fb = vec![0u32; fb_len];
        let mut zb = vec![100.0f32; zb_len];
        let mut tex = Texture::new(w, h).unwrap();
        tex.set_pixel(0, 0, 0x00FF_FFFFFF);

        unsafe {
            draw_span_nearest_simd(
                &mut fb, &mut zb, &tex, z, dz_dx, u_fix, v_fix, du_fix, dv_fix,
            );
        }
    }
}
