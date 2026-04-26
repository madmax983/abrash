import re

with open("benches/solarize_bench.rs", "r") as f:
    code = f.read()

code = code.replace("0x00FFFFFF", "0x00FF_FFFF")
code = code.replace("use std::arch::x86_64::*;", "use std::arch::x86_64::{_mm256_set1_epi8, _mm256_set1_epi32, __m256i, _mm256_loadu_si256, _mm256_sub_epi8, _mm256_cmpgt_epi8, _mm256_and_si256, _mm256_xor_si256, _mm256_storeu_si256};")
code = code.replace("let mut ptr = pixels.as_mut_ptr().cast::<__m256i>();", "let ptr = pixels.as_mut_ptr();")
code = code.replace("let mut chunk_ptr = ptr.add(i / 8);", "let mut chunk_ptr = ptr.add(i).cast::<__m256i>();")
code = code.replace("let mask_th = _mm256_set1_epi8((threshold as i8).wrapping_sub(128));", "let mask_th = _mm256_set1_epi8((threshold as i8).wrapping_sub(128u8 as i8));")

with open("benches/solarize_bench.rs", "w") as f:
    f.write(code)
