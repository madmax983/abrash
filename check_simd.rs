#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

fn main() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let a = _mm256_setzero_ps();
        let b = _mm256_castps_pd(a);
        let c = _mm256_castpd_ps(b);
    }
}
