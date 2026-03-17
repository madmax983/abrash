import os
import re

def fix_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # Match 0xXXXXXXXX (exactly 8 hex digits after 0x)
    # Replaces with 0xXXXX_XXXX
    content = re.sub(r'0x([0-9a-fA-F]{4})([0-9a-fA-F]{4})\b', r'0x\1_\2', content)

    # Some numbers mentioned in clippy output
    content = content.replace("123456789", "123_456_789")
    content = content.replace("987654321", "987_654_321")
    content = content.replace("1664525", "1_664_525")
    content = content.replace("1013904223", "1_013_904_223")
    content = content.replace("8388607.0", "8_388_607.0")
    content = content.replace("16777216.0", "16_777_216.0")

    # Wobble redundant closure
    content = content.replace("SOURCE_PIXELS.with(|source_pixels_cell| source_pixels_cell.take());", "SOURCE_PIXELS.with(std::cell::RefCell::take);")

    # CRT adding item
    content = content.replace("    use std::cell::RefCell;\n\n    thread_local! {", "")
    content = content.replace("pub fn apply_crt(fb: &mut Framebuffer, config: &CrtConfig) {", "use std::cell::RefCell;\n\nthread_local! {\n    static CRT_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };\n}\n\npub fn apply_crt(fb: &mut Framebuffer, config: &CrtConfig) {")

    # Rasterizer texture logic 1
    content = content.replace("""                    let idx = if is_pot {
                        // Note: We clamp to match the scalar implementation (draw_span_nearest / get_pixel_texel).
                        // Although wrapping is faster and standard for PoT, we must preserve rendering parity.
                        // The existing `draw_scanline_normal_mapped_simd` uses wrapping, but that creates
                        // an inconsistency with its own scalar fallback. We choose to be consistent with scalar here.
                        let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                        let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                        _mm256_or_si256(_mm256_sllv_epi32(v_c, shift_vec), u_c)
                    } else {
                        let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                        let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                        _mm256_add_epi32(_mm256_mullo_epi32(v_c, w_vec), u_c)
                    };""", """                    let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                    let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                    let idx = if is_pot {
                        // Note: We clamp to match the scalar implementation (draw_span_nearest / get_pixel_texel).
                        // Although wrapping is faster and standard for PoT, we must preserve rendering parity.
                        // The existing `draw_scanline_normal_mapped_simd` uses wrapping, but that creates
                        // an inconsistency with its own scalar fallback. We choose to be consistent with scalar here.
                        _mm256_or_si256(_mm256_sllv_epi32(v_c, shift_vec), u_c)
                    } else {
                        _mm256_add_epi32(_mm256_mullo_epi32(v_c, w_vec), u_c)
                    };""")

    # Rasterizer texture logic 2
    content = content.replace("""                let idx = if is_pot {
                    let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                    let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                    _mm256_or_si256(_mm256_sllv_epi32(v_c, shift_vec), u_c)
                } else {
                    let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                    let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                    _mm256_add_epi32(_mm256_mullo_epi32(v_c, w_vec), u_c)
                };""", """                let u_c = _mm256_min_epi32(_mm256_max_epi32(u_i, zero_i), max_x);
                let v_c = _mm256_min_epi32(_mm256_max_epi32(v_i, zero_i), max_y);
                let idx = if is_pot {
                    _mm256_or_si256(_mm256_sllv_epi32(v_c, shift_vec), u_c)
                } else {
                    _mm256_add_epi32(_mm256_mullo_epi32(v_c, w_vec), u_c)
                };""")

    # Math adding item
    content = content.replace("""        #[cfg(feature = "parallel")]
        {
            // Fallback to scalar for small inputs to avoid Rayon overhead
            if points.len() < 1024 {
                self.transform_points_uninit(points, output);
                return;
            }

            use rayon::prelude::*;
            // Chunk size of 4096 ensures we amortize task overhead and keep the AVX2
            // implementation fed with enough data to be efficient.
            const CHUNK_SIZE: usize = 4096;""", """        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            // Chunk size of 4096 ensures we amortize task overhead and keep the AVX2
            // implementation fed with enough data to be efficient.
            const CHUNK_SIZE: usize = 4096;

            // Fallback to scalar for small inputs to avoid Rayon overhead
            if points.len() < 1024 {
                self.transform_points_uninit(points, output);
                return;
            }""")
    content = content.replace("""    // NDC to screen coordinates
    // Clamp to [i32::MIN + 1, i32::MAX] to avoid integer overflow when negating i32::MIN.
    // We clamp the float value BEFORE casting to i32 to avoid Undefined Behavior with NaN/Inf.
    // 2147483520.0 is the largest f32 strictly less than i32::MAX + 1 that is exactly representable.
    const MAX_VAL: f32 = 2_147_483_520.0;
    const MIN_VAL: f32 = -2_147_483_520.0;""", """    // NDC to screen coordinates""")
    content = content.replace(""") -> ScreenPoint {
    // Perspective divide""", """) -> ScreenPoint {
    // Clamp to [i32::MIN + 1, i32::MAX] to avoid integer overflow when negating i32::MIN.
    // We clamp the float value BEFORE casting to i32 to avoid Undefined Behavior with NaN/Inf.
    // 2147483520.0 is the largest f32 strictly less than i32::MAX + 1 that is exactly representable.
    const MAX_VAL: f32 = 2_147_483_520.0;
    const MIN_VAL: f32 = -2_147_483_520.0;

    // Perspective divide""")

    # Framebuffer adding item
    content = content.replace("""    ) -> std::io::Result<()> {
        let converter =
            crate::ascii::AsciiConverter::new(self, crate::ascii::AsciiCharset::Standard);
        let content = converter.to_string();
        let mut file = std::fs::File::create(path)?;
        use std::io::Write;""", """    ) -> std::io::Result<()> {
        use std::io::Write;
        let converter =
            crate::ascii::AsciiConverter::new(self, crate::ascii::AsciiCharset::Standard);
        let content = converter.to_string();
        let mut file = std::fs::File::create(path)?;""")
    content = content.replace("""    ) -> std::io::Result<()> {
        let converter =
            crate::ascii::AsciiConverter::new(self, crate::ascii::AsciiCharset::Standard);
        let content = converter.to_colored_string();
        let mut file = std::fs::File::create(path)?;
        use std::io::Write;""", """    ) -> std::io::Result<()> {
        use std::io::Write;
        let converter =
            crate::ascii::AsciiConverter::new(self, crate::ascii::AsciiCharset::Standard);
        let content = converter.to_colored_string();
        let mut file = std::fs::File::create(path)?;""")

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

files_to_fix = [
    "src/post_process/bloom.rs",
    "src/post_process/filters.rs",
    "src/post_process/ssao.rs",
    "src/skybox.rs",
    "src/experimental/blueprint.rs",
    "src/experimental/glitch.rs",
    "src/experimental/raytracer.rs",
    "src/experimental/sharpen.rs",
    "src/experimental/vision.rs",
    "src/experimental/voronoi.rs",
    "src/experimental/voxel_explosion.rs",
    "src/experimental/wobble.rs",
    "src/experimental/crt.rs",
    "src/rasterizer/texture.rs",
    "src/math.rs",
    "src/framebuffer.rs",
    "src/rasterizer/core.rs",
    "src/rasterizer/pbr.rs",
    "src/rasterizer/tile.rs",
    "src/heat_vision.rs",
    "benches/run_film_grain.rs",
]

for f in files_to_fix:
    if os.path.exists(f):
        fix_file(f)
