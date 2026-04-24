#[cfg(test)]
mod havoc_tests {
    use crate::rasterizer::texture::{draw_span_bilinear_simd, draw_span_trilinear_simd};
    use abrash_core::texture::Texture;

    #[test]
    #[ignore = "👹 Havoc: Trigger SIMD buffer mismatch in trilinear"]
    fn havoc_texture_simd_buffer_mismatch_trilinear() {
        let mut fb = vec![0u32; 64];
        let mut zb = vec![100.0f32; 8]; // Intentionally small z-buffer!
        let mut tex = Texture::new(2, 2).unwrap();
        tex.generate_mipmaps();

        let z = 1.0;
        let dz_dx = 0.0;
        let u_fix = 0;
        let v_fix = 0;
        let du_fix = 0;
        let dv_fix = 0;

        unsafe {
            draw_span_trilinear_simd(
                &mut fb, &mut zb, &tex, z, dz_dx, u_fix, v_fix, du_fix, dv_fix, 1.0,
            );
        }
    }

    #[test]
    #[ignore = "👹 Havoc: Trigger SIMD buffer mismatch in bilinear"]
    fn havoc_texture_simd_buffer_mismatch_bilinear() {
        let mut fb = vec![0u32; 64];
        let mut zb = vec![100.0f32; 8];
        let tex = Texture::new(2, 2).unwrap();

        unsafe {
            draw_span_bilinear_simd(
                &mut fb, &mut zb, &tex, 1.0, 0.0, 0, 0, 0, 0,
            );
        }
    }
}
