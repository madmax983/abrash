import re

with open("examples/obj_viewer.rs", "r") as f:
    code = f.read()
code = code.replace("fn main() {", "fn main() -> Result<(), Box<dyn std::error::Error>> {")
with open("examples/obj_viewer.rs", "w") as f:
    f.write(code)

with open("examples/speed_lines_demo.rs", "r") as f:
    code = f.read()
code = code.replace("0xFF101030", "0xFF10_1030")
code = code.replace("0xFFFF0000", "0xFFFF_0000")
code = code.replace("0xFFFFFFFF", "0xFFFF_FFFF")
with open("examples/speed_lines_demo.rs", "w") as f:
    f.write(code)

with open("benches/solarize_bench.rs", "r") as f:
    code = f.read()
code = code.replace("let mut ptr = pixels.as_mut_ptr();", "let mut ptr = pixels.as_mut_ptr().cast::<__m256i>();")
code = code.replace("let mask_255 = _mm256_set1_epi8(-1); // 0xFF", "let _mask_255 = _mm256_set1_epi8(-1); // 0xFF")
code = code.replace("let th = threshold as i32;", "let th = i32::from(threshold);")
code = code.replace("let mask_th = _mm256_set1_epi8((threshold as i8).wrapping_sub(128));", "let mask_th = _mm256_set1_epi8((threshold as i8).wrapping_sub(128u8 as i8));")
code = code.replace("b.iter(|| {\n            apply_solarize(black_box(&mut fb), black_box(127));\n        })", "b.iter(|| {\n            apply_solarize(black_box(&mut fb), black_box(127));\n        });")
code = code.replace("b.iter(|| {\n            apply_solarize_branchless(black_box(&mut fb), black_box(127));\n        })", "b.iter(|| {\n            apply_solarize_branchless(black_box(&mut fb), black_box(127));\n        });")
code = code.replace("b.iter(|| {\n            apply_solarize_simd_avx2(black_box(fb.as_mut_slice()), black_box(127));\n        })", "b.iter(|| {\n            apply_solarize_simd_avx2(black_box(fb.as_mut_slice()), black_box(127));\n        });")
with open("benches/solarize_bench.rs", "w") as f:
    f.write(code)
