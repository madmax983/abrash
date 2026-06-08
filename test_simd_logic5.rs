#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
fn main() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let t = _mm256_set_epi32(100, 300, 600, 800, 100, 300, 600, 800);
        let v512 = _mm256_set1_epi32(512);
        let v0 = _mm256_set1_epi32(0);
        let v255 = _mm256_set1_epi32(255);
        let cmp_t_lt_512 = _mm256_cmpgt_epi32(v512, t);
        let mut b = _mm256_sub_epi32(t, v512);

        // if mask is true (cmp_t_lt_512), use 2nd operand (v0), else 1st operand (b)
        // Wait, _mm256_blendv_epi8(a, b, mask) -> if mask { b } else { a }
        let res = _mm256_blendv_epi8(b, v0, cmp_t_lt_512);

    }
}
