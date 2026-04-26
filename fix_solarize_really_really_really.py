import re

with open("benches/solarize_bench.rs", "r") as f:
    code = f.read()

code = code.replace("use std::arch::x86_64::{_mm256_set1_epi8, _mm256_set1_epi32, __m256i, _mm256_loadu_si256, _mm256_sub_epi8, _mm256_cmpgt_epi8, _mm256_and_si256, _mm256_xor_si256, _mm256_storeu_si256};", "use std::arch::x86_64::{__m256i, _mm256_and_si256, _mm256_cmpgt_epi8, _mm256_loadu_si256, _mm256_set1_epi32, _mm256_set1_epi8, _mm256_storeu_si256, _mm256_sub_epi8, _mm256_xor_si256};")
code = code.replace("let mask_th = _mm256_set1_epi8((threshold as i8).wrapping_sub(128));", "let th_val = _mm256_set1_epi8((threshold as i8).wrapping_sub(128u8 as i8));")
code = code.replace("let th_val = _mm256_set1_epi8((threshold as i8).wrapping_sub(128));", "let th_val = _mm256_set1_epi8((threshold as i8).wrapping_sub(128u8 as i8));")
code = code.replace("let p = _mm256_loadu_si256(chunk_ptr.cast());", "let p = _mm256_loadu_si256(ptr.cast());")
code = code.replace("_mm256_storeu_si256(chunk_ptr.cast(), res);", "_mm256_storeu_si256(ptr.cast(), res);")
code = code.replace("let ptr = pixels.as_mut_ptr();", "let mut ptr = pixels.as_mut_ptr();")
code = code.replace("ptr = ptr.add(1);", "ptr = ptr.add(8);")

with open("benches/solarize_bench.rs", "w") as f:
    f.write(code)
