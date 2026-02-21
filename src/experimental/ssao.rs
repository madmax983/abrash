//! Screen-Space Ambient Occlusion (SSAO) implementation.
//!
//! This module provides functions to apply SSAO to a rendered scene using the depth buffer.
//! SSAO simulates the darkening that occurs in corners and crevices where ambient light is occluded.

#![allow(warnings)]

use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec3};
use crate::zbuffer::ZBuffer;

const KERNEL_SIZE: usize = 16;
const NOISE_SIZE: usize = 4;

/// Generates a deterministic pseudo-random kernel for SSAO sampling.
fn generate_kernel() -> [Vec3; KERNEL_SIZE] {
    let mut kernel = [Vec3::default(); KERNEL_SIZE];
    let mut seed = 123456789;

    for (i, v) in kernel.iter_mut().enumerate() {
        let r1 = rand_f32(&mut seed) * 2.0 - 1.0; // x: -1..1
        let r2 = rand_f32(&mut seed) * 2.0 - 1.0; // y: -1..1
        let r3 = rand_f32(&mut seed); // z: 0..1 (hemisphere)

        let mut sample = Vec3::new(r1, r2, r3).normalize();

        // Scale samples to distribute them within the hemisphere
        let scale = i as f32 / KERNEL_SIZE as f32;
        let scale = lerp(0.1, 1.0, scale * scale);
        sample = sample * scale;

        *v = sample;
    }

    kernel
}

/// Generates a noise texture for kernel rotation.
fn generate_noise() -> [Vec3; NOISE_SIZE * NOISE_SIZE] {
    let mut noise = [Vec3::default(); NOISE_SIZE * NOISE_SIZE];
    let mut seed = 987654321;

    for v in noise.iter_mut() {
        let x = rand_f32(&mut seed) * 2.0 - 1.0;
        let y = rand_f32(&mut seed) * 2.0 - 1.0;
        *v = Vec3::new(x, y, 0.0).normalize();
    }

    noise
}

/// Simple Linear Congruential Generator for deterministic randomness.
fn rand_f32(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (*seed >> 9) as f32 / 8388607.0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Applies Screen-Space Ambient Occlusion to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify (darkened by occlusion).
/// * `zb` - The depth buffer (source of geometry).
/// * `proj` - The projection matrix used to render the scene.
/// * `radius` - Sampling radius in view space (e.g., 0.5).
/// * `bias` - Bias to prevent self-occlusion (e.g., 0.025).
/// * `intensity` - Strength of the effect (e.g., 1.0 - 3.0).
pub fn apply_ssao(
    fb: &mut Framebuffer,
    zb: &ZBuffer,
    proj: &Mat4,
    radius: f32,
    bias: f32,
    intensity: f32,
) {
    if fb.width() != zb.width() || fb.height() != zb.height() {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let kernel = generate_kernel();
    let noise = generate_noise();

    // Projection parameters for reconstruction
    // Based on standard perspective matrix construction
    let p00 = proj.m[0][0];
    let p11 = proj.m[1][1];
    let p22 = proj.m[2][2];
    let p32 = proj.m[3][2];
    // p23 should be -1.0 for perspective.

    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;

    let mut occlusion_buffer = vec![0.0f32; width * height];

    for y in 0..height {
        let noise_y = y % NOISE_SIZE;
        for x in 0..width {
            let noise_x = x % NOISE_SIZE;
            let noise_idx = noise_y * NOISE_SIZE + noise_x;
            let random_vec = noise[noise_idx];

            let depth_val = zb.get_depth(x as i32, y as i32).unwrap_or(1.0);

            // Skip background (assuming standard cleared depth is close to far plane)
            // But we might want AO on objects against far plane?
            // Usually far plane is 1.0 (or whatever clear value is).
            // Let's process everything except infinity if used.
            if depth_val >= 1.0 {
                // Assuming 1.0 is far plane
                continue;
            }

            // Reconstruct View Position
            // z_ndc = depth_val
            // z_view = -P32 / (z_ndc + P22)
            let z_view = -p32 / (depth_val + p22);

            // Reconstruct X, Y view
            // x_ndc = (x / half_width) - 1.0
            // y_ndc = 1.0 - (y / half_height)
            let x_ndc = (x as f32 / half_width) - 1.0;
            let y_ndc = 1.0 - (y as f32 / half_height);

            // x_view = x_ndc * (-z_view) / P00
            // y_view = y_ndc * (-z_view) / P11
            let x_view = x_ndc * (-z_view) / p00;
            let y_view = y_ndc * (-z_view) / p11;

            let pos_view = Vec3::new(x_view, y_view, z_view);

            // Calculate TBN basis
            // Random vector is in tangent space (XY plane).
            // We assume normal is roughly pointing towards camera (0, 0, 1) in view space for screen-space normals?
            // Actually, without normal buffer, we have to approximate.
            // A common trick is to use dFdx/dFdy to get face normal.
            // But here we'll just use the random vector to jitter the sample in a sphere
            // and maybe hemisphere pointing towards camera (z > 0 in tangent, z < 0 in view).
            // Since we don't have normals, we can't orient hemisphere to surface normal.
            // We will use a sphere kernel or assume normal is (0,0,1) (flat).
            // If we use sphere, we might sample inside the object.
            // If we use hemisphere along +Z (towards camera), it works for front-facing surfaces.
            // Let's assume Normal = (0, 0, 1) in View Space (towards camera).
            // T = random_vec - normal * dot(random, normal) ...
            // Since Normal is (0,0,1), Tangent is just (rnd.x, rnd.y, 0).
            // Bitangent = Cross(Normal, Tangent) = (-rnd.y, rnd.x, 0).
            // This is effectively just using random_vec as rotation in XY plane.

            // TBN matrix for Normal=(0,0,1)
            // T = (1, 0, 0), B = (0, 1, 0), N = (0, 0, 1) rotated by noise.
            // Effectively, we just rotate the kernel sample around Z axis.
            // random_vec has z=0.
            // We need to rotate kernel.xy by random_vec.xy?
            // Rotation matrix from random_vec:
            // [ rx  -ry  0 ]
            // [ ry   rx  0 ]
            // [ 0    0   1 ]

            let rx = random_vec.x;
            let ry = random_vec.y;

            let mut occlusion = 0.0;

            for k in 0..KERNEL_SIZE {
                let s = kernel[k];
                // Rotate sample
                // We want to rotate around Z axis.
                // rotated.x = s.x * rx - s.y * ry
                // rotated.y = s.x * ry + s.y * rx
                // rotated.z = s.z (unchanged)
                let rotated_sample = Vec3::new(
                    s.x * rx - s.y * ry,
                    s.x * ry + s.y * rx,
                    s.z, // Hemisphere Z is positive (towards camera if we map it to View +Z? Wait, View is -Z looking).
                         // If surface normal is towards camera (View +Z?), then sample should be in +Z.
                         // But View Space looks down -Z. Surface normal points to +Z (towards eye).
                         // So samples should have +Z component relative to surface.
                         // So if we add +Z to pos_view (which is negative Z), we move towards camera (closer).
                );

                let sample_pos = pos_view + rotated_sample * radius;

                // Project sample position
                let (sample_clip, sample_w) = proj.transform_point(sample_pos);

                // Perspective divide
                if sample_w > 0.0 {
                    let inv_w = 1.0 / sample_w;
                    let s_ndc_x = sample_clip.x * inv_w;
                    let s_ndc_y = sample_clip.y * inv_w;
                    // s_ndc_z is the depth of the sample itself
                    let _s_ndc_z = sample_clip.z * inv_w;

                    // Map to screen
                    // Screen Space [0, width], [0, height]
                    let s_screen_x = ((s_ndc_x + 1.0) * half_width) as i32;
                    let s_screen_y = ((1.0 - s_ndc_y) * half_height) as i32;

                    if s_screen_x >= 0
                        && s_screen_x < width as i32
                        && s_screen_y >= 0
                        && s_screen_y < height as i32
                    {
                        // Get depth from Z-buffer at sample position
                        let existing_depth = zb.get_depth(s_screen_x, s_screen_y).unwrap_or(1.0);

                        // Check occlusion:
                        // existing_depth is the geometry depth (NDC).
                        // s_ndc_z is the sample depth (NDC).
                        // In standard Z-buffer (0..1 or -1..1), closer objects have SMALLER Z?
                        // Wait.
                        // OpenGL default: -1 (near) to 1 (far).
                        // Abrash uses:
                        // Near plane maps to -1. Far maps to 1.
                        // So SMALLER Z is CLOSER.

                        // We want to check if the sample (which is around the surface) is BEHIND something.
                        // Sample is BEHIND if its depth (s_ndc_z) is GREATER than existing_depth.
                        // Wait, if sample is behind geometry, it is occluded.
                        // If sample is IN FRONT (smaller Z), it is not occluded.

                        // However, standard SSAO compares View Space depths usually.
                        // Let's use View Space.
                        // Reconstruct existing geometry View Z.
                        let existing_view_z = -p32 / (existing_depth + p22);
                        let sample_view_z = sample_pos.z; // This is the View Z of the sample point.

                        // View Z is negative. Closer to camera = larger (less negative, e.g. -1 > -10).
                        // Geometry is at existing_view_z.
                        // Sample is at sample_view_z.
                        // Occlusion happens if geometry is CLOSER (larger Z) than sample.
                        // i.e., existing_view_z > sample_view_z.

                        // Range check: If geometry is TOO close (blocking very far away), ignore.
                        // range_check = abs(existing - sample) < radius ? 1 : 0.

                        let range_check = (existing_view_z - sample_view_z).abs() < radius;

                        if existing_view_z >= sample_view_z + bias && range_check {
                            occlusion += 1.0;
                        }
                    }
                }
            }

            occlusion_buffer[y * width + x] = occlusion;
        }
    }

    // Normalize and Invert Occlusion
    // occlusion is count of occluded samples.
    // factor = 1.0 - (occlusion / KERNEL_SIZE)
    // We want output color = color * factor.
    // Higher occlusion -> Lower factor (darker).

    // Blur Pass (Box Blur 4x4)
    // We'll do a simple blur on occlusion_buffer before applying.
    let blurred = box_blur(&occlusion_buffer, width, height);

    // Apply to Framebuffer
    let pixels = fb.as_mut_slice();
    for (i, p) in pixels.iter_mut().enumerate() {
        let occ = blurred[i];
        let factor = 1.0 - (occ / KERNEL_SIZE as f32) * intensity;
        let factor = factor.clamp(0.0, 1.0);

        // Multiply RGB
        let r = ((*p >> 16) & 0xFF) as f32;
        let g = ((*p >> 8) & 0xFF) as f32;
        let b = (*p & 0xFF) as f32;

        let new_r = (r * factor) as u32;
        let new_g = (g * factor) as u32;
        let new_b = (b * factor) as u32;

        *p = (*p & 0xFF00_0000) | (new_r << 16) | (new_g << 8) | new_b;
    }
}

fn box_blur(src: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut dest = vec![0.0; src.len()];
    let radius = 2; // 5x5 kernel

    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0;
            let mut count = 0.0;

            for ky in -(radius as isize)..=radius as isize {
                for kx in -(radius as isize)..=radius as isize {
                    let ny = y as isize + ky;
                    let nx = x as isize + kx;

                    if ny >= 0 && ny < height as isize && nx >= 0 && nx < width as isize {
                        sum += src[ny as usize * width + nx as usize];
                        count += 1.0;
                    }
                }
            }
            dest[y * width + x] = sum / count;
        }
    }
    dest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::math::Mat4;
    use crate::zbuffer::ZBuffer;
    use std::f32::consts::PI;

    #[test]
    fn test_apply_ssao_darkens_occluded_pixels() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Fill FB with white
        fb.clear(0xFFFFFFFF);

        // Setup Z-buffer with a "corner"
        // Let's try to simulate a corner.
        // We need the depth values to be realistic for the projection matrix.
        // Let's use a standard projection.
        let proj = Mat4::perspective(PI / 2.0, 1.0, 0.1, 100.0);

        // Clear ZB to 1.0 (far)
        zb.clear(); // Clears to INFINITY
        for y in 0..height {
            for x in 0..width {
                // Initialize to far plane (1.0) for the test
                zb.test_and_set(x as i32, y as i32, 1.0);
            }
        }

        // Draw a flat wall at z=-10.0 (view space).
        // z_ndc = -P22 - P32/z_view
        // P22 = (100.1 / -99.9) approx -1.002
        // P32 = (2*100*0.1 / -99.9) approx -0.2
        // z_ndc = 1.002 - (-0.2 / -10) = 1.002 - 0.02 = 0.982
        let wall_depth = 0.982;

        // Draw a "post" in front of it at z=-9.6 (close to wall for SSAO).
        // z_ndc = 1.002 - (-0.2 / -9.6) = 1.002 - 0.0208 = 0.9812
        let post_depth = 0.9812;

        for y in 0..height {
            for x in 0..width {
                // Background wall (use test_and_set, it will pass because wall_depth < 1.0)
                zb.test_and_set(x as i32, y as i32, wall_depth);
            }
        }

        // Post in the center
        for y in 40..60 {
            for x in 40..60 {
                zb.test_and_set(x as i32, y as i32, post_depth);
            }
        }

        // Apply SSAO (radius 1.0 to cover gap 0.4)
        apply_ssao(&mut fb, &zb, &proj, 1.0, 0.001, 2.0);

        // Check pixels near the post (e.g., 39, 50).
        // They should be darkened because the post occludes the wall.
        // The post is at 40..60. Pixel 39 is just outside.
        // Samples from 39 will hit the post (depth 0.962) which is closer than wall (0.982).
        let p_occluded = fb.get_pixel(39, 50).unwrap();
        let p_unoccluded = fb.get_pixel(10, 50).unwrap();

        let lum_occluded = p_occluded & 0xFF;
        let lum_unoccluded = p_unoccluded & 0xFF;

        assert!(
            lum_occluded < lum_unoccluded,
            "Pixel near post should be darker (got {} vs {})",
            lum_occluded,
            lum_unoccluded
        );

        // Also ensure it didn't turn black (sanity check)
        assert!(lum_occluded > 0, "Pixel shouldn't be completely black");
    }
}
