//! Edge-case bounds testing for the Gouraud Rasterizer.
use crate::rasterizer::gouraud::draw_scanline_gouraud_simd_fast;

#[test]
fn havoc_gouraud_simd_buffer_mismatch() {
    let mut fb = vec![0u32; 64];
    let mut zb = vec![100.0f32; 8];

    let z_start = 1.0;
    let c_start = (0, 0, 0);
    let dz_dx = 0.0;
    let dc_dx = (0, 0, 0);

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if is_x86_feature_detected!("avx2") {
        unsafe {
            draw_scanline_gouraud_simd_fast(&mut fb, &mut zb, z_start, c_start, dz_dx, dc_dx);
        }
    }
}
