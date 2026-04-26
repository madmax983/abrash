import re

with open("benches/solarize_bench.rs", "r") as f:
    code = f.read()

code = code.replace("use std::arch::x86_64::{__m256i, _mm256_and_si256, _mm256_cmpgt_epi8, _mm256_loadu_si256, _mm256_set1_epi32, _mm256_set1_epi8, _mm256_storeu_si256, _mm256_sub_epi8, _mm256_xor_si256};", "use std::arch::x86_64::{_mm256_and_si256, _mm256_cmpgt_epi8, _mm256_loadu_si256, _mm256_set1_epi32, _mm256_set1_epi8, _mm256_storeu_si256, _mm256_sub_epi8, _mm256_xor_si256};")

with open("benches/solarize_bench.rs", "w") as f:
    f.write(code)
