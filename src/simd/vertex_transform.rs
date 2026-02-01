//! Batched vertex transformation using SIMD.
//!
//! This is where SIMD actually wins:
//! - Independent operations (no data dependencies)
//! - Math-heavy (matrix-vector multiply)
//! - No branches
//! - Sequential memory access

use crate::math::{Mat4, Vec3};
use std::arch::asm;

/// Transform a single vertex through a 4x4 matrix (scalar reference)
#[inline]
pub fn transform_vertex_scalar(matrix: &Mat4, v: Vec3) -> (Vec3, f32) {
    matrix.transform_point(v)
}

/// Transform 4 vertices in parallel using SSE
///
/// This is the money maker - completely independent operations processed in parallel.
/// Each vertex gets transformed by the same matrix simultaneously.
///
/// # Safety
/// Requires SSE support (guaranteed on x86-64)
#[cfg(target_feature = "sse")]
pub unsafe fn transform_vertices_4wide_sse(
    matrix: &Mat4,
    vertices: &[Vec3; 4],
) -> [(Vec3, f32); 4] {
    let mut results = [(Vec3::zero(), 0.0); 4];

    // STRATEGY:
    // Process all 4 vertices through the matrix in parallel
    // For each output component (x, y, z, w), we'll:
    // 1. Load all 4 vertex X values into xmm0 = [v0.x, v1.x, v2.x, v3.x]
    // 2. Broadcast matrix row element and multiply
    // 3. Accumulate across all matrix elements
    // 4. Store results

    unsafe {
        asm!(
            // ============================================================
            // Load all 4 vertices' X coordinates: [v0.x, v1.x, v2.x, v3.x]
            // ============================================================
            "movss xmm0, dword ptr [{v0}]",       // v0.x
            "movss xmm1, dword ptr [{v1}]",       // v1.x
            "movss xmm2, dword ptr [{v2}]",       // v2.x
            "movss xmm3, dword ptr [{v3}]",       // v3.x
            "unpcklps xmm0, xmm1",                 // [v0.x, v1.x, 0, 0]
            "unpcklps xmm2, xmm3",                 // [v2.x, v3.x, 0, 0]
            "movlhps xmm0, xmm2",                  // xmm0 = [v0.x, v1.x, v2.x, v3.x]

            // Load all 4 vertices' Y coordinates
            "movss xmm1, dword ptr [{v0} + 4]",
            "movss xmm2, dword ptr [{v1} + 4]",
            "movss xmm3, dword ptr [{v2} + 4]",
            "movss xmm4, dword ptr [{v3} + 4]",
            "unpcklps xmm1, xmm2",
            "unpcklps xmm3, xmm4",
            "movlhps xmm1, xmm3",                  // xmm1 = [v0.y, v1.y, v2.y, v3.y]

            // Load all 4 vertices' Z coordinates
            "movss xmm2, dword ptr [{v0} + 8]",
            "movss xmm3, dword ptr [{v1} + 8]",
            "movss xmm4, dword ptr [{v2} + 8]",
            "movss xmm5, dword ptr [{v3} + 8]",
            "unpcklps xmm2, xmm3",
            "unpcklps xmm4, xmm5",
            "movlhps xmm2, xmm4",                  // xmm2 = [v0.z, v1.z, v2.z, v3.z]

            // Now we have:
            // xmm0 = [v0.x, v1.x, v2.x, v3.x]
            // xmm1 = [v0.y, v1.y, v2.y, v3.y]
            // xmm2 = [v0.z, v1.z, v2.z, v3.z]

            // ============================================================
            // Compute output X for all 4 vertices
            // out.x = m[0][0]*v.x + m[1][0]*v.y + m[2][0]*v.z + m[3][0]
            // ============================================================

            // m[0][0] * [v0.x, v1.x, v2.x, v3.x]
            "movss xmm3, dword ptr [{mat}]",       // m[0][0]
            "shufps xmm3, xmm3, 0",                // Broadcast m[0][0]
            "mulps xmm3, xmm0",                    // m[0][0] * vx
            "movaps xmm8, xmm3",                   // xmm8 = accumulator for out.x

            // m[1][0] * [v0.y, v1.y, v2.y, v3.y]
            "movss xmm3, dword ptr [{mat} + 16]",  // m[1][0]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm1",
            "addps xmm8, xmm3",

            // m[2][0] * [v0.z, v1.z, v2.z, v3.z]
            "movss xmm3, dword ptr [{mat} + 32]",  // m[2][0]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm2",
            "addps xmm8, xmm3",

            // + m[3][0]
            "movss xmm3, dword ptr [{mat} + 48]",  // m[3][0]
            "shufps xmm3, xmm3, 0",
            "addps xmm8, xmm3",
            // xmm8 now has [out0.x, out1.x, out2.x, out3.x]

            // ============================================================
            // Compute output Y for all 4 vertices
            // ============================================================
            "movss xmm3, dword ptr [{mat} + 4]",   // m[0][1]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm0",
            "movaps xmm9, xmm3",

            "movss xmm3, dword ptr [{mat} + 20]",  // m[1][1]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm1",
            "addps xmm9, xmm3",

            "movss xmm3, dword ptr [{mat} + 36]",  // m[2][1]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm2",
            "addps xmm9, xmm3",

            "movss xmm3, dword ptr [{mat} + 52]",  // m[3][1]
            "shufps xmm3, xmm3, 0",
            "addps xmm9, xmm3",
            // xmm9 now has [out0.y, out1.y, out2.y, out3.y]

            // ============================================================
            // Compute output Z for all 4 vertices
            // ============================================================
            "movss xmm3, dword ptr [{mat} + 8]",   // m[0][2]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm0",
            "movaps xmm10, xmm3",

            "movss xmm3, dword ptr [{mat} + 24]",  // m[1][2]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm1",
            "addps xmm10, xmm3",

            "movss xmm3, dword ptr [{mat} + 40]",  // m[2][2]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm2",
            "addps xmm10, xmm3",

            "movss xmm3, dword ptr [{mat} + 56]",  // m[3][2]
            "shufps xmm3, xmm3, 0",
            "addps xmm10, xmm3",
            // xmm10 now has [out0.z, out1.z, out2.z, out3.z]

            // ============================================================
            // Compute output W for all 4 vertices
            // ============================================================
            "movss xmm3, dword ptr [{mat} + 12]",  // m[0][3]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm0",
            "movaps xmm11, xmm3",

            "movss xmm3, dword ptr [{mat} + 28]",  // m[1][3]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm1",
            "addps xmm11, xmm3",

            "movss xmm3, dword ptr [{mat} + 44]",  // m[2][3]
            "shufps xmm3, xmm3, 0",
            "mulps xmm3, xmm2",
            "addps xmm11, xmm3",

            "movss xmm3, dword ptr [{mat} + 60]",  // m[3][3]
            "shufps xmm3, xmm3, 0",
            "addps xmm11, xmm3",
            // xmm11 now has [out0.w, out1.w, out2.w, out3.w]

            // ============================================================
            // Store results
            // We have 4 output vectors in xmm8-xmm11 (x, y, z, w)
            // Need to transpose and store
            // ============================================================

            // Store vertex 0: extract first element from each register
            "movss dword ptr [{r0}], xmm8",        // out0.x
            "movss dword ptr [{r0} + 4], xmm9",    // out0.y
            "movss dword ptr [{r0} + 8], xmm10",   // out0.z
            "movss dword ptr [{r0} + 12], xmm11",  // out0.w

            // Extract second elements (shift and store)
            "shufps xmm8, xmm8, 0x39",             // Rotate to get second element
            "shufps xmm9, xmm9, 0x39",
            "shufps xmm10, xmm10, 0x39",
            "shufps xmm11, xmm11, 0x39",

            "movss dword ptr [{r1}], xmm8",
            "movss dword ptr [{r1} + 4], xmm9",
            "movss dword ptr [{r1} + 8], xmm10",
            "movss dword ptr [{r1} + 12], xmm11",

            // Third elements
            "shufps xmm8, xmm8, 0x39",
            "shufps xmm9, xmm9, 0x39",
            "shufps xmm10, xmm10, 0x39",
            "shufps xmm11, xmm11, 0x39",

            "movss dword ptr [{r2}], xmm8",
            "movss dword ptr [{r2} + 4], xmm9",
            "movss dword ptr [{r2} + 8], xmm10",
            "movss dword ptr [{r2} + 12], xmm11",

            // Fourth elements
            "shufps xmm8, xmm8, 0x39",
            "shufps xmm9, xmm9, 0x39",
            "shufps xmm10, xmm10, 0x39",
            "shufps xmm11, xmm11, 0x39",

            "movss dword ptr [{r3}], xmm8",
            "movss dword ptr [{r3} + 4], xmm9",
            "movss dword ptr [{r3} + 8], xmm10",
            "movss dword ptr [{r3} + 12], xmm11",

            mat = in(reg) &matrix.m as *const [[f32; 4]; 4],
            v0 = in(reg) &vertices[0] as *const Vec3,
            v1 = in(reg) &vertices[1] as *const Vec3,
            v2 = in(reg) &vertices[2] as *const Vec3,
            v3 = in(reg) &vertices[3] as *const Vec3,
            r0 = in(reg) &mut results[0] as *mut (Vec3, f32),
            r1 = in(reg) &mut results[1] as *mut (Vec3, f32),
            r2 = in(reg) &mut results[2] as *mut (Vec3, f32),
            r3 = in(reg) &mut results[3] as *mut (Vec3, f32),
            out("xmm0") _,
            out("xmm1") _,
            out("xmm2") _,
            out("xmm3") _,
            out("xmm4") _,
            out("xmm5") _,
            out("xmm8") _,
            out("xmm9") _,
            out("xmm10") _,
            out("xmm11") _,
            options(nostack),
        );
    }

    results
}

/// Transform a batch of vertices (any count) using SIMD where possible
#[cfg(target_feature = "sse")]
pub fn transform_vertices_batch(matrix: &Mat4, vertices: &[Vec3]) -> Vec<(Vec3, f32)> {
    let mut results = Vec::with_capacity(vertices.len());

    let num_batches = vertices.len() / 4;
    let remainder = vertices.len() % 4;

    // Process 4-wide batches
    for i in 0..num_batches {
        let batch_start = i * 4;
        let batch = [
            vertices[batch_start],
            vertices[batch_start + 1],
            vertices[batch_start + 2],
            vertices[batch_start + 3],
        ];

        let transformed = unsafe { transform_vertices_4wide_sse(matrix, &batch) };
        results.extend_from_slice(&transformed);
    }

    // Handle remainder with scalar code
    for i in (num_batches * 4)..vertices.len() {
        results.push(transform_vertex_scalar(matrix, vertices[i]));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    fn approx_eq_vec3(a: Vec3, b: Vec3, epsilon: f32) -> bool {
        (a.x - b.x).abs() < epsilon && (a.y - b.y).abs() < epsilon && (a.z - b.z).abs() < epsilon
    }

    fn approx_eq_f32(a: f32, b: f32, epsilon: f32) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    #[cfg(target_feature = "sse")]
    fn test_transform_4wide_identity() {
        let matrix = Mat4::identity();
        let vertices = [
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(7.0, 8.0, 9.0),
            Vec3::new(10.0, 11.0, 12.0),
        ];

        let results = unsafe { transform_vertices_4wide_sse(&matrix, &vertices) };

        for i in 0..4 {
            assert!(
                approx_eq_vec3(results[i].0, vertices[i], EPSILON),
                "Vertex {} mismatch: {:?} != {:?}",
                i,
                results[i].0,
                vertices[i]
            );
            assert!(
                approx_eq_f32(results[i].1, 1.0, EPSILON),
                "W component should be 1.0"
            );
        }
    }

    #[test]
    #[cfg(target_feature = "sse")]
    fn test_transform_4wide_vs_scalar() {
        let matrix = Mat4::rotation_y(0.5);
        let vertices = [
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-1.0, 0.5, 2.0),
            Vec3::new(0.0, -1.0, 1.0),
            Vec3::new(3.0, 3.0, 3.0),
        ];

        let simd_results = unsafe { transform_vertices_4wide_sse(&matrix, &vertices) };

        for i in 0..4 {
            let scalar_result = transform_vertex_scalar(&matrix, vertices[i]);

            assert!(
                approx_eq_vec3(simd_results[i].0, scalar_result.0, EPSILON),
                "SIMD vs Scalar mismatch at vertex {}: {:?} != {:?}",
                i,
                simd_results[i].0,
                scalar_result.0
            );
            assert!(
                approx_eq_f32(simd_results[i].1, scalar_result.1, EPSILON),
                "W mismatch at vertex {}: {} != {}",
                i,
                simd_results[i].1,
                scalar_result.1
            );
        }
    }

    #[test]
    #[cfg(target_feature = "sse")]
    fn test_transform_batch() {
        let matrix = Mat4::translation(1.0, 2.0, 3.0);
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::new(3.0, 3.0, 3.0),
            Vec3::new(4.0, 4.0, 4.0),
            Vec3::new(5.0, 5.0, 5.0),
        ];

        let results = transform_vertices_batch(&matrix, &vertices);

        assert_eq!(results.len(), vertices.len());

        for i in 0..vertices.len() {
            let expected = transform_vertex_scalar(&matrix, vertices[i]);
            assert!(
                approx_eq_vec3(results[i].0, expected.0, EPSILON),
                "Batch transform mismatch at {}: {:?} != {:?}",
                i,
                results[i].0,
                expected.0
            );
        }
    }
}
