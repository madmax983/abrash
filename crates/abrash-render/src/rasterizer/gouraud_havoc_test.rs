//! Edge-case bounds testing for the Gouraud Rasterizer.
use crate::rasterizer::gouraud::{
    GouraudGradients, GouraudSpanStart, draw_scanline_gouraud_simd_fast,
};

#[test]
fn havoc_gouraud_simd_buffer_mismatch() {
    let mut fb = vec![0u32; 64];
    // Intentionally smaller zb buffer. Since SIMD iterates based on fb.len(),
    // it will read past the end of zb.
    let mut zb = vec![100.0f32; 8];

    let z_start = 1.0;
    let c_start = (0, 0, 0);
    let dz_dx = 0.0;
    let dc_dx = (0, 0, 0);

    unsafe {
        draw_scanline_gouraud_simd_fast(
            &mut fb,
            &mut zb,
            GouraudSpanStart { z_start, c_start },
            &GouraudGradients { dz_dx, dc_dx },
        );
    }
}
