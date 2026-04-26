import re

with open("benches/solarize_bench.rs", "r") as f:
    code = f.read()

code = code.replace("use std::arch::x86_64::{_mm256_set1_epi8, _mm256_set1_epi32, __m256i, _mm256_loadu_si256, _mm256_sub_epi8, _mm256_cmpgt_epi8, _mm256_and_si256, _mm256_xor_si256, _mm256_storeu_si256};", "use std::arch::x86_64::*;")
code = code.replace("});;;", "})")
code = code.replace("});;", "})")
code = code.replace("let ptr = pixels.as_mut_ptr();", "let mut ptr = pixels.as_mut_ptr() as *mut __m256i;")
code = code.replace("let mut ptr = pixels.as_mut_ptr() as *mut __m256i;", "let mut ptr = pixels.as_mut_ptr().cast::<__m256i>();")
code = code.replace("0x00FFFFFF", "0x00FF_FFFF")
code = code.replace("let mask_255 = _mm256_set1_epi8(-1); // 0xFF", "let _mask_255 = _mm256_set1_epi8(-1); // 0xFF")
code = code.replace("let th = threshold as i32;", "let th = i32::from(threshold);")
code = code.replace("let mask_th = _mm256_set1_epi8((threshold as i8).wrapping_sub(128));", "let mask_th = _mm256_set1_epi8((threshold as i8).wrapping_sub(128u8 as i8));")
code = code.replace("let p = _mm256_loadu_si256(ptr);", "let p = _mm256_loadu_si256(ptr.cast());")
code = code.replace("_mm256_storeu_si256(ptr, res);", "_mm256_storeu_si256(ptr.cast(), res);")
code = code.replace("let mut chunk_ptr = ptr.add(i).cast::<__m256i>();", "let mut chunk_ptr = ptr.add(i / 8).cast::<__m256i>();")


with open("benches/solarize_bench.rs", "w") as f:
    f.write(code)

with open("examples/cloth_demo.rs", "r") as f:
    code = f.read()

code = code.replace("use std::error::Error;", "")

with open("examples/cloth_demo.rs", "w") as f:
    f.write(code)

with open("examples/selective_color_demo.rs", "r") as f:
    code = f.read()

code = code.replace("SelectiveColorApp::new().unwrap()", "SelectiveColorApp::new()")

with open("examples/selective_color_demo.rs", "w") as f:
    f.write(code)
