#[cfg(test)]
mod simd_safety_tests {
    use super::*;
    use crate::rasterizer::tile::rasterize_scanline_simd;

    #[test]
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    fn test_rasterize_scanline_simd_misaligned_buffers() {
        // Allocate buffers with extra space to force misalignment
        let mut pixels_vec = vec![0u32; 128];
        let mut depths_vec = vec![f32::INFINITY; 128];

        // Force pixels to be 32-byte aligned
        let pixels_addr = pixels_vec.as_ptr() as usize;
        let pixels_offset = (32 - (pixels_addr % 32)) % 32;
        let pixels_idx = pixels_offset / 4;
        let pixels_slice = &mut pixels_vec[pixels_idx..pixels_idx + 64];

        // Force depths to be MISALIGNED relative to 32 bytes (offset 4 bytes)
        // depths_slice[0] should be at address ending in ...04, ...24, etc.
        // relative to a 32-byte aligned address.
        // We want depths alignment != pixels alignment.
        // pixels is aligned (offset 0).
        // depths should have offset 4.
        let depths_addr = depths_vec.as_ptr() as usize;
        // Current alignment of depths_vec
        let current_align = depths_addr % 32;
        // We want (depths_addr + offset * 4) % 32 == 4
        // (current_align + offset * 4) % 32 == 4
        // Try offset = 0..32 until we find one.
        let mut depths_idx = 0;
        for i in 0..32 {
            if ((depths_addr + i * 4) % 32) == 4 {
                depths_idx = i;
                break;
            }
        }
        let depths_slice = &mut depths_vec[depths_idx..depths_idx + 64];

        // Verify alignment assumptions
        assert_eq!((pixels_slice.as_ptr() as usize) % 32, 0, "Pixels should be aligned");
        assert_eq!((depths_slice.as_ptr() as usize) % 32, 4, "Depths should be misaligned");

        // Run the function
        // This should crash if aligned stores are used on depths
        rasterize_scanline_simd(
            pixels_slice,
            depths_slice,
            0.5, // z_start
            0.01, // dz_dx
            0xFFFFFFFF, // color
        );
    }
}
