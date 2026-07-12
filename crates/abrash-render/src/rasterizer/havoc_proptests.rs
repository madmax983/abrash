use crate::rasterizer::texture::draw_span_nearest_simd;
use crate::rasterizer::texture::draw_span_trilinear_simd;
use abrash_core::texture::Texture;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]
    #[test]
    #[ignore = "👺 Havoc: Fuzzing draw_span_nearest_simd bounds"]
    fn fuzz_draw_span_nearest_simd(
        z in any::<f32>(),
        dz_dx in any::<f32>(),
        u_fix in any::<i32>(),
        v_fix in any::<i32>(),
        du_fix in any::<i32>(),
        dv_fix in any::<i32>(),
    ) {
        let mut fb = vec![0u32; 100];
        let mut zb = vec![100.0f32; 100];
        let mut tex = Texture::new(10, 10).unwrap();
        tex.set_pixel(0, 0, 0x00FF_FFFFFF);

        // draw_span_nearest_simd unchecked memory bounds mapping exploit
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            unsafe {
                draw_span_nearest_simd(
                    &mut fb, &mut zb, &tex, z, dz_dx, u_fix, v_fix, du_fix, dv_fix,
                );
            }
        }));
    }
}
