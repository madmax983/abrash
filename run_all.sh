#!/bin/bash
cat << 'PYEOF' > step1.py
with open('crates/abrash-render/src/post_process/filters.rs', 'a') as f:
    f.write('''
#[test]
fn test_apply_exposure() {
    let mut fb = Framebuffer::new(2, 2).unwrap();
    fb.as_mut_slice()[0] = 0xFF00_0000 | (100 << 16) | (100 << 8) | 100;
    apply_exposure(&mut fb, 2.0);
    assert_eq!(fb.as_mut_slice()[0], 0xFF00_0000 | (200 << 16) | (200 << 8) | 200);
}
''')

with open('benches/post_process.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'benchmark_color_adjust,\n);',
    'benchmark_color_adjust,\n    benchmark_exposure,\n);'
)

content = content.replace(
    'fn benchmark_color_adjust(c: &mut Criterion) {',
    'fn benchmark_exposure(c: &mut Criterion) {\n    let mut fb = Framebuffer::new(1920, 1080).unwrap();\n    c.bench_function("apply_exposure 1080p", |b| {\n        b.iter(|| {\n            post_process::filters::apply_exposure(black_box(&mut fb), black_box(2.0));\n        });\n    });\n}\n\nfn benchmark_color_adjust(c: &mut Criterion) {'
)

with open('benches/post_process.rs', 'w') as f:
    f.write(content)
PYEOF
python3 step1.py

cat << 'PYEOF' > step3.py
with open('crates/abrash-render/src/post_process/filters.rs', 'r') as f:
    content = f.read()

new_func = '''
pub fn apply_exposure(fb: &mut Framebuffer, exposure: f32) {
    let pixels = fb.as_mut_slice();
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            let simd_len = pixels.len() & !7;
            unsafe { simd::apply_exposure_avx2(&mut pixels[..simd_len], exposure) };
            apply_exposure_scalar(&mut pixels[simd_len..], exposure);
            return;
        }
    }
    apply_exposure_scalar(pixels, exposure);
}

fn apply_exposure_scalar(pixels: &mut [u32], exposure: f32) {
    let scale = (exposure * 256.0) as u32;
    for pixel in pixels.iter_mut() {
        let r = (((*pixel >> 16) & 0xFF) * scale) >> 8;
        let g = (((*pixel >> 8) & 0xFF) * scale) >> 8;
        let b = ((*pixel & 0xFF) * scale) >> 8;
        let r = r.min(255);
        let g = g.min(255);
        let b = b.min(255);
        *pixel = 0xFF00_0000 | (r << 16) | (g << 8) | b;
    }
}
'''
content = content.replace('pub fn apply_grayscale(fb: &mut Framebuffer) {', new_func + '\npub fn apply_grayscale(fb: &mut Framebuffer) {')

simd_func = '''
    pub unsafe fn apply_exposure_avx2(pixels: &mut [u32], exposure: f32) {
        let scale = _mm256_set1_ps(exposure);
        let mask = _mm256_set1_epi32(0xFF);
        let alpha_mask = _mm256_set1_epi32(0xFF00_0000_u32 as i32);
        for chunk in pixels.chunks_exact_mut(8) {
            let mut p = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);
            let r = _mm256_cvtepi32_ps(_mm256_and_si256(_mm256_srli_epi32(p, 16), mask));
            let g = _mm256_cvtepi32_ps(_mm256_and_si256(_mm256_srli_epi32(p, 8), mask));
            let b = _mm256_cvtepi32_ps(_mm256_and_si256(p, mask));
            let r = _mm256_mul_ps(r, scale);
            let g = _mm256_mul_ps(g, scale);
            let b = _mm256_mul_ps(b, scale);
            let max_val = _mm256_set1_ps(255.0);
            let r = _mm256_cvtps_epi32(_mm256_min_ps(r, max_val));
            let g = _mm256_cvtps_epi32(_mm256_min_ps(g, max_val));
            let b = _mm256_cvtps_epi32(_mm256_min_ps(b, max_val));
            p = _mm256_or_si256(alpha_mask, _mm256_or_si256(_mm256_slli_epi32(r, 16), _mm256_or_si256(_mm256_slli_epi32(g, 8), b)));
            _mm256_storeu_si256(chunk.as_mut_ptr() as *mut __m256i, p);
        }
    }
'''
content = content.replace('    pub unsafe fn apply_grayscale_avx2(pixels: &mut [u32]) {', simd_func + '\n    pub unsafe fn apply_grayscale_avx2(pixels: &mut [u32]) {')

with open('crates/abrash-render/src/post_process/filters.rs', 'w') as f:
    f.write(content)
PYEOF
python3 step3.py

cat << 'PYEOF' > step3_2.py
with open('crates/abrash-render/src/post_process/mod.rs', 'r') as f:
    content = f.read()

content = content.replace('pub use self::filters::*;', 'pub use self::filters::*;\npub use filters::apply_exposure;')

with open('crates/abrash-render/src/post_process/mod.rs', 'w') as f:
    f.write(content)
PYEOF
python3 step3_2.py
