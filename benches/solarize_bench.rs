use abrash_core::framebuffer::Framebuffer;
use abrash_render::post_process::filters::apply_solarize;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn apply_solarize_branchless(fb: &mut Framebuffer, threshold: u8) {
    let pixels = fb.as_mut_slice();
    let th = i32::from(threshold);
    for pixel in pixels.iter_mut() {
        let p = *pixel;
        let a = p & 0xFF00_0000;
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        let new_r = r ^ ((((th - r as i32) >> 31) as u32) & 0xFF);
        let new_g = g ^ ((((th - g as i32) >> 31) as u32) & 0xFF);
        let new_b = b ^ ((((th - b as i32) >> 31) as u32) & 0xFF);

        *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
    }
}

pub fn apply_solarize_simd_avx2(pixels: &mut [u32], threshold: u8) {
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            let len = pixels.len();
            let simd_len = len & !7;
            unsafe {
                use std::arch::x86_64::{
                    _mm256_and_si256, _mm256_cmpgt_epi8, _mm256_loadu_si256, _mm256_set1_epi8,
                    _mm256_set1_epi32, _mm256_storeu_si256, _mm256_sub_epi8, _mm256_xor_si256,
                };
                let th_val = _mm256_set1_epi8((threshold as i8).wrapping_sub(128u8 as i8)); // Offset by 128 for signed compare
                let _mask_255 = _mm256_set1_epi8(-1); // 0xFF
                let mask_rgb = _mm256_set1_epi32(0x00FF_FFFF);

                let mut ptr = pixels.as_mut_ptr();
                for _ in 0..(simd_len / 8) {
                    let p = _mm256_loadu_si256(ptr.cast());

                    // To compare unsigned 8-bit integers, we can subtract 128 (toggle MSB) and use signed compare
                    let p_offset = _mm256_sub_epi8(p, _mm256_set1_epi8(-128));
                    let cmp = _mm256_cmpgt_epi8(p_offset, th_val);

                    // We only want to invert RGB channels, not Alpha
                    let cmp_rgb = _mm256_and_si256(cmp, mask_rgb);

                    // XOR with 0xFF inverts the bits. XOR with 0 does nothing.
                    let res = _mm256_xor_si256(p, cmp_rgb);

                    _mm256_storeu_si256(ptr.cast(), res);
                    ptr = ptr.add(8);
                }
            }

            // Tail
            let tail_slice = &mut pixels[simd_len..];
            let th = i32::from(threshold);
            for pixel in tail_slice.iter_mut() {
                let p = *pixel;
                let a = p & 0xFF00_0000;
                let r = (p >> 16) & 0xFF;
                let g = (p >> 8) & 0xFF;
                let b = p & 0xFF;

                let new_r = r ^ ((((th - r as i32) >> 31) as u32) & 0xFF);
                let new_g = g ^ ((((th - g as i32) >> 31) as u32) & 0xFF);
                let new_b = b ^ ((((th - b as i32) >> 31) as u32) & 0xFF);

                *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
            }
            return;
        }
    }

    // Fallback
    let th = i32::from(threshold);
    for pixel in pixels.iter_mut() {
        let p = *pixel;
        let a = p & 0xFF00_0000;
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        let new_r = r ^ ((((th - r as i32) >> 31) as u32) & 0xFF);
        let new_g = g ^ ((((th - g as i32) >> 31) as u32) & 0xFF);
        let new_b = b ^ ((((th - b as i32) >> 31) as u32) & 0xFF);

        *pixel = a | (new_r << 16) | (new_g << 8) | new_b;
    }
}

fn benchmark_solarize(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    for i in 0..(1920 * 1080) {
        fb.as_mut_slice()[i] = (i as u32) | 0xFF00_0000;
    }

    c.bench_function("solarize", |b| {
        b.iter(|| {
            apply_solarize(black_box(&mut fb), black_box(127));
        });
    });

    c.bench_function("solarize_branchless", |b| {
        b.iter(|| {
            apply_solarize_branchless(black_box(&mut fb), black_box(127));
        });
    });

    c.bench_function("solarize_simd_avx2", |b| {
        b.iter(|| {
            apply_solarize_simd_avx2(black_box(fb.as_mut_slice()), black_box(127));
        });
    });
}

criterion_group!(benches, benchmark_solarize);
criterion_main!(benches);
