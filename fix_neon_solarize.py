import re

with open("benches/solarize_bench.rs", "r") as f:
    code = f.read()

code = code.replace("0x00FFFFFF", "0x00FF_FFFF")
code = code.replace("use std::arch::x86_64::*;", "use std::arch::x86_64::{_mm256_set1_epi8, _mm256_set1_epi32, __m256i, _mm256_loadu_si256, _mm256_sub_epi8, _mm256_cmpgt_epi8, _mm256_and_si256, _mm256_xor_si256, _mm256_storeu_si256};")
code = code.replace("let mut ptr = pixels.as_mut_ptr() as *mut __m256i;", "let mut ptr = pixels.as_mut_ptr().cast::<__m256i>();")
code = code.replace("let mask_255 = _mm256_set1_epi8(-1); // 0xFF", "let _mask_255 = _mm256_set1_epi8(-1); // 0xFF")
code = code.replace("let th = threshold as i32;", "let th = i32::from(threshold);")
code = code.replace("let mask_th = _mm256_set1_epi8((threshold as i8).wrapping_sub(128));", "let mask_th = _mm256_set1_epi8((threshold as i8).wrapping_sub(128u8 as i8));")

code = code.replace("b.iter(|| {\n            apply_solarize(black_box(&mut fb), black_box(127));\n        })", "b.iter(|| {\n            apply_solarize(black_box(&mut fb), black_box(127));\n        });")
code = code.replace("b.iter(|| {\n            apply_solarize_branchless(black_box(&mut fb), black_box(127));\n        })", "b.iter(|| {\n            apply_solarize_branchless(black_box(&mut fb), black_box(127));\n        });")
code = code.replace("b.iter(|| {\n            apply_solarize_simd_avx2(black_box(fb.as_mut_slice()), black_box(127));\n        })", "b.iter(|| {\n            apply_solarize_simd_avx2(black_box(fb.as_mut_slice()), black_box(127));\n        });")

with open("benches/solarize_bench.rs", "w") as f:
    f.write(code)

with open("benches/neon_outline_bench.rs", "r") as f:
    code = f.read()

code = code.replace("0xFFFFFFFF", "0xFFFF_FFFF")
code = code.replace("0xFF000000", "0xFF00_0000")

with open("benches/neon_outline_bench.rs", "w") as f:
    f.write(code)
