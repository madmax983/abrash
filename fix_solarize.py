import re

with open("benches/solarize_bench.rs", "r") as f:
    code = f.read()

code = code.replace("let mask_255 = _mm256_set1_epi8(-1); // 0xFF", "let _mask_255 = _mm256_set1_epi8(-1); // 0xFF")
code = code.replace("let th = threshold as i32;", "let th = i32::from(threshold);")
code = code.replace("let th = threshold as i32;", "let th = i32::from(threshold);")
code = code.replace("let th = threshold as i32;", "let th = i32::from(threshold);")
code = code.replace("let mut ptr = pixels.as_mut_ptr() as *mut __m256i;", "let mut ptr = pixels.as_mut_ptr().cast::<__m256i>();")
code = code.replace("b.iter(|| {\n            apply_solarize(black_box(&mut fb), black_box(127));\n        })", "b.iter(|| {\n            apply_solarize(black_box(&mut fb), black_box(127));\n        });")
code = code.replace("b.iter(|| {\n            apply_solarize_branchless(black_box(&mut fb), black_box(127));\n        })", "b.iter(|| {\n            apply_solarize_branchless(black_box(&mut fb), black_box(127));\n        });")
code = code.replace("b.iter(|| {\n            apply_solarize_simd_avx2(black_box(fb.as_mut_slice()), black_box(127));\n        })", "b.iter(|| {\n            apply_solarize_simd_avx2(black_box(fb.as_mut_slice()), black_box(127));\n        });")

with open("benches/solarize_bench.rs", "w") as f:
    f.write(code)
