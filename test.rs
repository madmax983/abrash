fn main() {
    let p_y = 397070100000000.0f32;
    let p_z = 9.910708e17f32;
    let m11 = -4.6952516e28f32;
    let m21 = 7.794001e37f32;

    let res_mul = p_y * m11 + p_z * m21;
    println!("Scalar add+mul: {}", res_mul);

    unsafe {
        use std::arch::x86_64::*;
        let vz = _mm256_set1_ps(p_z);
        let m21_v = _mm256_set1_ps(m21);
        let vy = _mm256_set1_ps(p_y);
        let m11_v = _mm256_set1_ps(m11);
        let vx = _mm256_setzero_ps();
        let m01 = _mm256_setzero_ps();
        let m31 = _mm256_setzero_ps();

        let res = _mm256_fmadd_ps(
            vz, m21_v,
            _mm256_fmadd_ps(
                vy, m11_v,
                _mm256_fmadd_ps(vx, m01, m31)
            )
        );

        let mut out = [0.0f32; 8];
        _mm256_storeu_ps(out.as_mut_ptr(), res);
        println!("FMA: {}", out[0]);
    }
}
