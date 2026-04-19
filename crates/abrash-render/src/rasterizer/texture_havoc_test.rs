use crate::rasterizer::texture::draw_span_nearest_simd;
use abrash_core::texture::Texture;

#[test]
#[ignore = "👹 Havoc: Trigger SIMD out of bounds access directly"]
fn havoc_texture_overflow_simd() {
    let mut fb = vec![0u32; 64];
    let mut zb = vec![100.0f32; 64];
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0xFFFFFFFF);

    let z = 1.0;
    let dz_dx = 0.0;

    let u_fix = i32::MAX - 100;
    let v_fix = i32::MAX - 100;
    let du_fix = 286331154; // Causes wrap around
    let dv_fix = 286331154;

    // This bypasses the scalar bounds checks done before calling SIMD,
    // simulating when `u_fix` and `v_fix` wrap around into large positive values in the AVX loop.
    unsafe {
        draw_span_nearest_simd(
            &mut fb, &mut zb, &tex, z, dz_dx, u_fix, v_fix, du_fix, dv_fix,
        );
    }
}
