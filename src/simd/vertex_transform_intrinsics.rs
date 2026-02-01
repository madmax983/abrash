//! Vertex transformation using SSE intrinsics instead of raw assembly.
//!
//! Key insight: Let LLVM handle register allocation and optimization.
//! We provide the SIMD structure, LLVM does the hard parts.

use crate::math::{Mat4, Vec3};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Transform 4 vertices using SSE intrinsics (LLVM-friendly)
///
/// Strategy: Use intrinsics instead of raw asm! so the compiler can:
/// - Optimize register allocation
/// - Inline aggressively
/// - Eliminate redundant operations
/// - Choose optimal instruction sequences
///
/// # Safety
/// Requires SSE support (guaranteed on x86-64)
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse")]
pub unsafe fn transform_vertices_4wide_intrinsics(
    matrix: &Mat4,
    vertices: &[Vec3; 4],
) -> [(Vec3, f32); 4] {
    // Load matrix rows as __m128
    let m_row0 = _mm_loadu_ps(&matrix.m[0][0]);
    let m_row1 = _mm_loadu_ps(&matrix.m[1][0]);
    let m_row2 = _mm_loadu_ps(&matrix.m[2][0]);
    let m_row3 = _mm_loadu_ps(&matrix.m[3][0]);

    // Transpose vertices from AOS to SOA: gather X, Y, Z components
    // vx = [v0.x, v1.x, v2.x, v3.x]
    let vx = _mm_set_ps(vertices[3].x, vertices[2].x, vertices[1].x, vertices[0].x);
    let vy = _mm_set_ps(vertices[3].y, vertices[2].y, vertices[1].y, vertices[0].y);
    let vz = _mm_set_ps(vertices[3].z, vertices[2].z, vertices[1].z, vertices[0].z);

    // Transform all 4 vertices in parallel
    // For each output component, we broadcast the matrix element and multiply

    // Compute X components for all 4 vertices
    let mut out_x = _mm_mul_ps(_mm_set1_ps(matrix.m[0][0]), vx);
    out_x = _mm_add_ps(out_x, _mm_mul_ps(_mm_set1_ps(matrix.m[1][0]), vy));
    out_x = _mm_add_ps(out_x, _mm_mul_ps(_mm_set1_ps(matrix.m[2][0]), vz));
    out_x = _mm_add_ps(out_x, _mm_set1_ps(matrix.m[3][0]));

    // Compute Y components
    let mut out_y = _mm_mul_ps(_mm_set1_ps(matrix.m[0][1]), vx);
    out_y = _mm_add_ps(out_y, _mm_mul_ps(_mm_set1_ps(matrix.m[1][1]), vy));
    out_y = _mm_add_ps(out_y, _mm_mul_ps(_mm_set1_ps(matrix.m[2][1]), vz));
    out_y = _mm_add_ps(out_y, _mm_set1_ps(matrix.m[3][1]));

    // Compute Z components
    let mut out_z = _mm_mul_ps(_mm_set1_ps(matrix.m[0][2]), vx);
    out_z = _mm_add_ps(out_z, _mm_mul_ps(_mm_set1_ps(matrix.m[1][2]), vy));
    out_z = _mm_add_ps(out_z, _mm_mul_ps(_mm_set1_ps(matrix.m[2][2]), vz));
    out_z = _mm_add_ps(out_z, _mm_set1_ps(matrix.m[3][2]));

    // Compute W components
    let mut out_w = _mm_mul_ps(_mm_set1_ps(matrix.m[0][3]), vx);
    out_w = _mm_add_ps(out_w, _mm_mul_ps(_mm_set1_ps(matrix.m[1][3]), vy));
    out_w = _mm_add_ps(out_w, _mm_mul_ps(_mm_set1_ps(matrix.m[2][3]), vz));
    out_w = _mm_add_ps(out_w, _mm_set1_ps(matrix.m[3][3]));

    // Store results (SOA to AOS)
    let mut results = [(Vec3::zero(), 0.0); 4];

    // Extract and store each vertex
    let mut temp_x = [0.0f32; 4];
    let mut temp_y = [0.0f32; 4];
    let mut temp_z = [0.0f32; 4];
    let mut temp_w = [0.0f32; 4];

    _mm_storeu_ps(temp_x.as_mut_ptr(), out_x);
    _mm_storeu_ps(temp_y.as_mut_ptr(), out_y);
    _mm_storeu_ps(temp_z.as_mut_ptr(), out_z);
    _mm_storeu_ps(temp_w.as_mut_ptr(), out_w);

    for i in 0..4 {
        results[i] = (Vec3::new(temp_x[i], temp_y[i], temp_z[i]), temp_w[i]);
    }

    results
}

/// SSE3 version using HADDPS for potentially better performance
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse3")]
pub unsafe fn transform_vertices_4wide_sse3(
    matrix: &Mat4,
    vertices: &[Vec3; 4],
) -> [(Vec3, f32); 4] {
    // Same as SSE version but compiler might use better instructions
    transform_vertices_4wide_intrinsics(matrix, vertices)
}

/// FMA version for even better performance (if available)
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "fma")]
pub unsafe fn transform_vertices_4wide_fma(
    matrix: &Mat4,
    vertices: &[Vec3; 4],
) -> [(Vec3, f32); 4] {
    let vx = _mm_set_ps(vertices[3].x, vertices[2].x, vertices[1].x, vertices[0].x);
    let vy = _mm_set_ps(vertices[3].y, vertices[2].y, vertices[1].y, vertices[0].y);
    let vz = _mm_set_ps(vertices[3].z, vertices[2].z, vertices[1].z, vertices[0].z);

    // Use FMA: a*b + c in one instruction (faster + more accurate)
    let mut out_x = _mm_mul_ps(_mm_set1_ps(matrix.m[0][0]), vx);
    out_x = _mm_fmadd_ps(_mm_set1_ps(matrix.m[1][0]), vy, out_x);
    out_x = _mm_fmadd_ps(_mm_set1_ps(matrix.m[2][0]), vz, out_x);
    out_x = _mm_add_ps(out_x, _mm_set1_ps(matrix.m[3][0]));

    let mut out_y = _mm_mul_ps(_mm_set1_ps(matrix.m[0][1]), vx);
    out_y = _mm_fmadd_ps(_mm_set1_ps(matrix.m[1][1]), vy, out_y);
    out_y = _mm_fmadd_ps(_mm_set1_ps(matrix.m[2][1]), vz, out_y);
    out_y = _mm_add_ps(out_y, _mm_set1_ps(matrix.m[3][1]));

    let mut out_z = _mm_mul_ps(_mm_set1_ps(matrix.m[0][2]), vx);
    out_z = _mm_fmadd_ps(_mm_set1_ps(matrix.m[1][2]), vy, out_z);
    out_z = _mm_fmadd_ps(_mm_set1_ps(matrix.m[2][2]), vz, out_z);
    out_z = _mm_add_ps(out_z, _mm_set1_ps(matrix.m[3][2]));

    let mut out_w = _mm_mul_ps(_mm_set1_ps(matrix.m[0][3]), vx);
    out_w = _mm_fmadd_ps(_mm_set1_ps(matrix.m[1][3]), vy, out_w);
    out_w = _mm_fmadd_ps(_mm_set1_ps(matrix.m[2][3]), vz, out_w);
    out_w = _mm_add_ps(out_w, _mm_set1_ps(matrix.m[3][3]));

    let mut results = [(Vec3::zero(), 0.0); 4];

    let mut temp_x = [0.0f32; 4];
    let mut temp_y = [0.0f32; 4];
    let mut temp_z = [0.0f32; 4];
    let mut temp_w = [0.0f32; 4];

    _mm_storeu_ps(temp_x.as_mut_ptr(), out_x);
    _mm_storeu_ps(temp_y.as_mut_ptr(), out_y);
    _mm_storeu_ps(temp_z.as_mut_ptr(), out_z);
    _mm_storeu_ps(temp_w.as_mut_ptr(), out_w);

    for i in 0..4 {
        results[i] = (Vec3::new(temp_x[i], temp_y[i], temp_z[i]), temp_w[i]);
    }

    results
}

/// Batch transform using intrinsics
#[cfg(target_arch = "x86_64")]
pub fn transform_vertices_batch_intrinsics(
    matrix: &Mat4,
    vertices: &[Vec3],
) -> Vec<(Vec3, f32)> {
    let mut results = Vec::with_capacity(vertices.len());

    let num_batches = vertices.len() / 4;

    // Process 4-wide batches with intrinsics
    for i in 0..num_batches {
        let batch_start = i * 4;
        let batch = [
            vertices[batch_start],
            vertices[batch_start + 1],
            vertices[batch_start + 2],
            vertices[batch_start + 3],
        ];

        #[cfg(target_feature = "sse")]
        let transformed = unsafe { transform_vertices_4wide_intrinsics(matrix, &batch) };

        #[cfg(not(target_feature = "sse"))]
        let transformed = {
            use crate::simd::vertex_transform::transform_vertex_scalar;
            [
                transform_vertex_scalar(matrix, batch[0]),
                transform_vertex_scalar(matrix, batch[1]),
                transform_vertex_scalar(matrix, batch[2]),
                transform_vertex_scalar(matrix, batch[3]),
            ]
        };

        results.extend_from_slice(&transformed);
    }

    // Handle remainder with scalar code
    for i in (num_batches * 4)..vertices.len() {
        results.push(matrix.transform_point(vertices[i]));
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
    #[cfg(target_arch = "x86_64")]
    fn test_intrinsics_vs_scalar() {
        let matrix = Mat4::perspective(1.047, 800.0 / 600.0, 0.1, 100.0);
        let vertices = [
            Vec3::new(1.0, 2.0, -5.0),
            Vec3::new(-1.0, 3.0, -6.0),
            Vec3::new(2.0, -1.0, -4.0),
            Vec3::new(0.5, 1.5, -5.5),
        ];

        let intrinsics_results = unsafe { transform_vertices_4wide_intrinsics(&matrix, &vertices) };

        for i in 0..4 {
            let scalar_result = matrix.transform_point(vertices[i]);

            assert!(
                approx_eq_vec3(intrinsics_results[i].0, scalar_result.0, EPSILON),
                "Intrinsics vs Scalar mismatch at vertex {}: {:?} != {:?}",
                i,
                intrinsics_results[i].0,
                scalar_result.0
            );
            assert!(
                approx_eq_f32(intrinsics_results[i].1, scalar_result.1, EPSILON),
                "W mismatch at vertex {}: {} != {}",
                i,
                intrinsics_results[i].1,
                scalar_result.1
            );
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_batch_intrinsics() {
        let matrix = Mat4::translation(1.0, 2.0, 3.0);
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::new(3.0, 3.0, 3.0),
            Vec3::new(4.0, 4.0, 4.0),
            Vec3::new(5.0, 5.0, 5.0),
        ];

        let results = transform_vertices_batch_intrinsics(&matrix, &vertices);

        assert_eq!(results.len(), vertices.len());

        for i in 0..vertices.len() {
            let expected = matrix.transform_point(vertices[i]);
            assert!(
                approx_eq_vec3(results[i].0, expected.0, EPSILON),
                "Batch intrinsics mismatch at {}: {:?} != {:?}",
                i,
                results[i].0,
                expected.0
            );
        }
    }
}
