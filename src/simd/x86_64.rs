//! x86-64 SIMD implementations using inline assembly.
//!
//! SSE2 is baseline requirement (all x86-64 CPUs have it).
//! AVX used when available for wider operations.

use crate::math::{Mat4, Vec3};
use std::arch::asm;

/// Vec3 dot product using SSE inline assembly.
///
/// Computes: self.x * other.x + self.y * other.y + self.z * other.z
///
/// Using SSE we can:
/// 1. Load both Vec3s into XMM registers
/// 2. Multiply in parallel (MULPS)
/// 3. Horizontal add to get final scalar
///
/// # Safety
/// Requires SSE support (guaranteed on x86-64)
#[inline]
#[cfg(target_feature = "sse")]
pub unsafe fn dot_sse(a: Vec3, b: Vec3) -> f32 {
    let result: f32;

    // Use SSE to compute dot product
    // We'll load the vectors, multiply, and sum
    // SAFETY: This function is marked unsafe and requires SSE support
    unsafe {
        asm!(
            // Load a.x, a.y, a.z into xmm0 with fourth component as 0
            "movss xmm0, dword ptr [rdi]",      // xmm0 = [a.x, 0, 0, 0]
            "movss xmm1, dword ptr [rdi + 4]",  // xmm1 = [a.y, 0, 0, 0]
            "movss xmm2, dword ptr [rdi + 8]",  // xmm2 = [a.z, 0, 0, 0]
            "unpcklps xmm0, xmm1",               // xmm0 = [a.x, a.y, 0, 0]
            "movlhps xmm0, xmm2",                // xmm0 = [a.x, a.y, a.z, 0]

            // Load b.x, b.y, b.z into xmm1 with fourth component as 0
            "movss xmm1, dword ptr [rsi]",      // xmm1 = [b.x, 0, 0, 0]
            "movss xmm2, dword ptr [rsi + 4]",  // xmm2 = [b.y, 0, 0, 0]
            "movss xmm3, dword ptr [rsi + 8]",  // xmm3 = [b.z, 0, 0, 0]
            "unpcklps xmm1, xmm2",               // xmm1 = [b.x, b.y, 0, 0]
            "movlhps xmm1, xmm3",                // xmm1 = [b.x, b.y, b.z, 0]

            // Multiply
            "mulps xmm0, xmm1",                  // xmm0 = [a.x*b.x, a.y*b.y, a.z*b.z, 0]

            // Horizontal add: sum first 3 components
            // xmm0 = [p0, p1, p2, 0]
            "movaps xmm1, xmm0",                 // xmm1 = [p0, p1, p2, 0]
            "shufps xmm1, xmm0, 0x4E",           // xmm1 = [p2, 0, p0, p1]
            "addps xmm0, xmm1",                  // xmm0 = [p0+p2, p1+0, ?, ?]
            "movaps xmm1, xmm0",
            "shufps xmm1, xmm0, 0x01",           // xmm1 = [p1+0, ?, ?, ?]
            "addss xmm0, xmm1",                  // xmm0[0] = p0+p2+p1

            // Move result to output
            "movss {result}, xmm0",

            result = out(xmm_reg) result,
            in("rdi") &a as *const Vec3,
            in("rsi") &b as *const Vec3,
            out("xmm0") _,
            out("xmm1") _,
            out("xmm2") _,
            out("xmm3") _,
            options(nostack, pure, readonly),
        );
    }

    result
}

/// SSE4.1 version using DPPS (dot product instruction)
///
/// Single instruction to compute dot product!
#[inline]
#[cfg(target_feature = "sse4.1")]
pub unsafe fn dot_sse41(a: Vec3, b: Vec3) -> f32 {
    let result: f32;

    asm!(
        // Load vectors (simpler version - we'll use DPPS which masks)
        "movss xmm0, dword ptr [rdi]",
        "movss xmm1, dword ptr [rdi + 4]",
        "insertps xmm0, xmm1, 0x10",         // Insert a.y at position 1
        "movss xmm1, dword ptr [rdi + 8]",
        "insertps xmm0, xmm1, 0x20",         // Insert a.z at position 2

        "movss xmm1, dword ptr [rsi]",
        "movss xmm2, dword ptr [rsi + 4]",
        "insertps xmm1, xmm2, 0x10",
        "movss xmm2, dword ptr [rsi + 8]",
        "insertps xmm1, xmm2, 0x20",

        // DPPS: mask 0x71 = 0111_0001
        // Upper nibble (0111): use first 3 components for multiply
        // Lower nibble (0001): store result in first component only
        "dpps xmm0, xmm1, 0x71",

        "movss {result}, xmm0",

        result = out(xmm_reg) result,
        in("rdi") &a as *const Vec3,
        in("rsi") &b as *const Vec3,
        out("xmm0") _,
        out("xmm1") _,
        out("xmm2") _,
        options(nostack, pure),
    );

    result
}

/// Mat4 multiplication using SSE inline assembly.
///
/// Classic SSE matrix multiply: process rows/columns in parallel.
/// Each row is a 4-wide vector perfect for SSE registers.
///
/// # Safety
/// Requires SSE support (guaranteed on x86-64)
#[inline]
#[cfg(target_feature = "sse")]
pub unsafe fn mat4_mul_sse(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut result = Mat4 { m: [[0.0; 4]; 4] };

    // Strategy:
    // - Load each row of B into registers (b_row0, b_row1, b_row2, b_row3)
    // - For each row of A, broadcast each element and multiply with corresponding B row
    // - Accumulate results

    // SAFETY: This function is marked unsafe and requires SSE support
    unsafe {
        asm!(
        // Load all 4 rows of matrix B into xmm4-xmm7
        "movups xmm4, [{b} + 0]",   // B row 0
        "movups xmm5, [{b} + 16]",  // B row 1
        "movups xmm6, [{b} + 32]",  // B row 2
        "movups xmm7, [{b} + 48]",  // B row 3

        // Process each row of A
        // Row 0 of result
        "movss xmm0, [{a} + 0]",    // a[0][0]
        "shufps xmm0, xmm0, 0",     // Broadcast a[0][0]
        "mulps xmm0, xmm4",          // a[0][0] * B_row0

        "movss xmm1, [{a} + 4]",    // a[0][1]
        "shufps xmm1, xmm1, 0",
        "mulps xmm1, xmm5",          // a[0][1] * B_row1
        "addps xmm0, xmm1",

        "movss xmm2, [{a} + 8]",    // a[0][2]
        "shufps xmm2, xmm2, 0",
        "mulps xmm2, xmm6",
        "addps xmm0, xmm2",

        "movss xmm3, [{a} + 12]",   // a[0][3]
        "shufps xmm3, xmm3, 0",
        "mulps xmm3, xmm7",
        "addps xmm0, xmm3",

        "movups [{result} + 0], xmm0",  // Store result row 0

        // Row 1 of result
        "movss xmm0, [{a} + 16]",
        "shufps xmm0, xmm0, 0",
        "mulps xmm0, xmm4",

        "movss xmm1, [{a} + 20]",
        "shufps xmm1, xmm1, 0",
        "mulps xmm1, xmm5",
        "addps xmm0, xmm1",

        "movss xmm2, [{a} + 24]",
        "shufps xmm2, xmm2, 0",
        "mulps xmm2, xmm6",
        "addps xmm0, xmm2",

        "movss xmm3, [{a} + 28]",
        "shufps xmm3, xmm3, 0",
        "mulps xmm3, xmm7",
        "addps xmm0, xmm3",

        "movups [{result} + 16], xmm0",

        // Row 2 of result
        "movss xmm0, [{a} + 32]",
        "shufps xmm0, xmm0, 0",
        "mulps xmm0, xmm4",

        "movss xmm1, [{a} + 36]",
        "shufps xmm1, xmm1, 0",
        "mulps xmm1, xmm5",
        "addps xmm0, xmm1",

        "movss xmm2, [{a} + 40]",
        "shufps xmm2, xmm2, 0",
        "mulps xmm2, xmm6",
        "addps xmm0, xmm2",

        "movss xmm3, [{a} + 44]",
        "shufps xmm3, xmm3, 0",
        "mulps xmm3, xmm7",
        "addps xmm0, xmm3",

        "movups [{result} + 32], xmm0",

        // Row 3 of result
        "movss xmm0, [{a} + 48]",
        "shufps xmm0, xmm0, 0",
        "mulps xmm0, xmm4",

        "movss xmm1, [{a} + 52]",
        "shufps xmm1, xmm1, 0",
        "mulps xmm1, xmm5",
        "addps xmm0, xmm1",

        "movss xmm2, [{a} + 56]",
        "shufps xmm2, xmm2, 0",
        "mulps xmm2, xmm6",
        "addps xmm0, xmm2",

        "movss xmm3, [{a} + 60]",
        "shufps xmm3, xmm3, 0",
        "mulps xmm3, xmm7",
        "addps xmm0, xmm3",

        "movups [{result} + 48], xmm0",

        a = in(reg) &a.m as *const [[f32; 4]; 4],
        b = in(reg) &b.m as *const [[f32; 4]; 4],
        result = in(reg) &mut result.m as *mut [[f32; 4]; 4],
        out("xmm0") _,
        out("xmm1") _,
        out("xmm2") _,
        out("xmm3") _,
        out("xmm4") _,
        out("xmm5") _,
        out("xmm6") _,
        out("xmm7") _,
        options(nostack),
        );
    }

    result
}

/// Optimized reciprocal approximation using SSE RCPSS
///
/// Much faster than division for cases where we can tolerate slight imprecision.
/// Typical use: perspective divide in rendering (1/w)
///
/// # Safety
/// Requires SSE support (guaranteed on x86-64)
#[inline]
#[cfg(target_feature = "sse")]
pub unsafe fn rcp_ss(x: f32) -> f32 {
    let result: f32;
    // SAFETY: This function is marked unsafe and requires SSE support
    unsafe {
        asm!(
            "movss xmm0, {x}",
            "rcpss xmm0, xmm0",      // Fast reciprocal approximation
            "movss {result}, xmm0",
            x = in(xmm_reg) x,
            result = out(xmm_reg) result,
            options(nostack, pure, nomem),
        );
    }
    result
}

/// 4-wide parallel reciprocal (for processing 4 pixels at once)
#[inline]
#[cfg(target_feature = "sse")]
pub unsafe fn rcp_ps(x: [f32; 4]) -> [f32; 4] {
    let mut result = [0.0f32; 4];
    // SAFETY: This function is marked unsafe and requires SSE support
    unsafe {
        asm!(
            "movups xmm0, [{x}]",
            "rcpps xmm0, xmm0",
            "movups [{result}], xmm0",
            x = in(reg) &x as *const [f32; 4],
            result = in(reg) &mut result as *mut [f32; 4],
            out("xmm0") _,
            options(nostack),
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    #[cfg(target_feature = "sse")]
    fn test_dot_sse() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);

        let expected = 1.0 * 4.0 + 2.0 * 5.0 + 3.0 * 6.0; // 32.0
        let result = unsafe { dot_sse(a, b) };

        assert!(
            approx_eq(result, expected, EPSILON),
            "SSE dot product mismatch: {} != {}",
            result,
            expected
        );
    }

    #[test]
    #[cfg(target_feature = "sse4.1")]
    fn test_dot_sse41() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);

        let expected = 32.0;
        let result = unsafe { dot_sse41(a, b) };

        assert!(
            approx_eq(result, expected, EPSILON),
            "SSE4.1 dot product mismatch: {} != {}",
            result,
            expected
        );
    }

    #[test]
    #[cfg(target_feature = "sse")]
    fn test_mat4_mul_sse() {
        let a = Mat4::rotation_y(0.5);
        let b = Mat4::translation(1.0, 2.0, 3.0);

        let expected = a.mul(&b);
        let result = unsafe { mat4_mul_sse(&a, &b) };

        for i in 0..4 {
            for j in 0..4 {
                assert!(
                    approx_eq(result.m[i][j], expected.m[i][j], EPSILON),
                    "Mat4 SSE mul mismatch at [{}, {}]: {} != {}",
                    i,
                    j,
                    result.m[i][j],
                    expected.m[i][j]
                );
            }
        }
    }

    #[test]
    #[cfg(target_feature = "sse")]
    fn test_rcp_ss() {
        let x = 4.0;
        let expected = 0.25;
        let result = unsafe { rcp_ss(x) };

        // RCPSS is approximate - allow larger epsilon
        assert!(
            approx_eq(result, expected, 0.001),
            "RCPSS mismatch: {} != {} (input: {})",
            result,
            expected,
            x
        );
    }

    #[test]
    #[cfg(target_feature = "sse")]
    fn test_rcp_ps() {
        let x = [2.0, 4.0, 8.0, 16.0];
        let expected = [0.5, 0.25, 0.125, 0.0625];
        let result = unsafe { rcp_ps(x) };

        for i in 0..4 {
            assert!(
                approx_eq(result[i], expected[i], 0.001),
                "RCPPS mismatch at [{}]: {} != {}",
                i,
                result[i],
                expected[i]
            );
        }
    }
}
