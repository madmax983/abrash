//! Depth of Field effect.
//!
//! Simulates camera focus by blurring pixels based on their depth in the Z-buffer.

use super::blur::{box_blur_horizontal, box_blur_vertical};
use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;
use std::cell::RefCell;

thread_local! {
    static DOF_CONTEXT: RefCell<DofContext> = RefCell::new(DofContext::default());
}

#[derive(Default)]
struct DofContext {
    blurred_buffer: Vec<u32>,
    scratch_buffer: Vec<u32>,
    acc_buffer: Vec<i32>,
}

/// Configuration for the Depth of Field effect.
#[derive(Clone, Copy, Debug)]
pub struct DepthOfFieldConfig {
    /// The depth at which objects are perfectly in focus (0.0 - 1.0 in non-linear z-buffer space).
    pub focus_dist: f32,
    /// The range of depth that remains reasonably sharp.
    pub focus_range: f32,
    /// The radius of the blur for out-of-focus areas.
    pub blur_radius: u32,
}

impl Default for DepthOfFieldConfig {
    fn default() -> Self {
        Self {
            focus_dist: 0.5,
            focus_range: 0.1,
            blur_radius: 5,
        }
    }
}

/// Applies depth of field effect.
///
/// This simulates a camera lens where objects at `focus_dist` are sharp, and objects
/// farther away (in either direction) become increasingly blurred.
///
/// # Arguments
/// * `fb` - The framebuffer (modified in-place).
/// * `zb` - The depth buffer.
/// * `config` - Configuration for the Depth of Field effect.
///
/// # Examples
///
/// ```
/// use abrash::framebuffer::Framebuffer;
/// use abrash::zbuffer::ZBuffer;
/// use abrash::post_process::dof::apply_depth_of_field;
///
/// let width = 800;
/// let height = 600;
/// let mut fb = Framebuffer::new(width, height).unwrap();
/// let mut zb = ZBuffer::new(width, height).unwrap();
///
/// // Render scene...
///
/// // Apply Depth of Field
/// // Focus on objects at depth 5.0 (in View Space, converted to Z-Buffer space appropriately)
/// // Note: ZBuffer typically stores non-linear depth.
/// let config = abrash::post_process::dof::DepthOfFieldConfig { focus_dist: 0.5, focus_range: 0.1, blur_radius: 5 };
/// apply_depth_of_field(&mut fb, &zb, &config);
/// ```
pub fn apply_depth_of_field(fb: &mut Framebuffer, zb: &ZBuffer, config: &DepthOfFieldConfig) {
    if config.blur_radius == 0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let needed_size = width * height;
    let acc_needed_size = width * 3;

    DOF_CONTEXT.with(|ctx_ref| {
        let mut ctx = ctx_ref.borrow_mut();

        if ctx.blurred_buffer.len() < needed_size {
            ctx.blurred_buffer.resize(needed_size, 0);
        }
        if ctx.scratch_buffer.len() < needed_size {
            ctx.scratch_buffer.resize(needed_size, 0);
        }
        if ctx.acc_buffer.len() < acc_needed_size {
            ctx.acc_buffer.resize(acc_needed_size, 0);
        }

        let DofContext {
            blurred_buffer,
            scratch_buffer,
            acc_buffer,
        } = &mut *ctx;

        let blurred_slice = &mut blurred_buffer[..needed_size];
        let scratch_slice = &mut scratch_buffer[..needed_size];
        let acc_slice = &mut acc_buffer[..acc_needed_size];
        let original_pixels = fb.as_mut_slice();

        // 1. Create blurred copy
        // Horizontal pass: original -> scratch
        box_blur_horizontal(
            original_pixels,
            scratch_slice,
            width,
            height,
            config.blur_radius,
        );
        // Vertical pass: scratch -> blurred
        box_blur_vertical(
            scratch_slice,
            blurred_slice,
            acc_slice,
            width,
            height,
            config.blur_radius,
        );

        // 2. Blend based on depth
        // We iterate over the original buffer and the blurred buffer
        let zb_slice = zb.as_slice();

        // Ensure we don't go out of bounds if buffers mismatch (though they shouldn't)
        let len = original_pixels
            .len()
            .min(zb_slice.len())
            .min(blurred_slice.len());

        for i in 0..len {
            let depth = zb_slice[i];

            // Skip infinite depth (skybox) if desired, or treat as far.
            // ZBuffer init is INFINITY. If depth is INFINITY, it's background.
            // If focus is near, background is blurred.
            // If focus is far, background is sharp?
            // Let's treat INFINITY as far (e.g. 1.0 or just use large number)
            let z = if depth.is_infinite() { 1000.0 } else { depth };

            let dist = (z - config.focus_dist).abs();

            // Calculate blur factor (0.0 = sharp, 1.0 = full blur)
            // If dist < range, factor = 0.
            // If dist > range, factor increases.
            // Simple linear falloff:
            let factor = ((dist - config.focus_range) / config.focus_range).clamp(0.0, 1.0);

            if factor > 0.0 {
                let orig = original_pixels[i];
                let blur = blurred_slice[i];

                // Convert factor to 0-256 fixed point
                let factor_fixed = (factor * 256.0) as u32;
                let inv_factor = 256 - factor_fixed;

                let r_o = (orig >> 16) & 0xFF;
                let g_o = (orig >> 8) & 0xFF;
                let b_o = orig & 0xFF;

                let r_b = (blur >> 16) & 0xFF;
                let g_b = (blur >> 8) & 0xFF;
                let b_b = blur & 0xFF;

                let r_new = (r_o * inv_factor + r_b * factor_fixed) >> 8;
                let g_new = (g_o * inv_factor + g_b * factor_fixed) >> 8;
                let b_new = (b_o * inv_factor + b_b * factor_fixed) >> 8;

                // Preserve alpha
                original_pixels[i] = (orig & 0xFF00_0000) | (r_new << 16) | (g_new << 8) | b_new;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_dof_changes_pixels() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Fill with white
        fb.clear(0xFFFFFFFF);

        // Fill ZBuffer:
        // Left half: depth 1.0 (focus)
        // Right half: depth 10.0 (out of focus)
        for y in 0..height {
            for x in 0..width {
                let depth = if x < width / 2 { 1.0 } else { 10.0 };
                zb.test_and_set(x as i32, y as i32, depth);
            }
        }

        // Apply pattern to FB to see blur
        // Checkerboard
        for y in 0..height {
            for x in 0..width {
                if (x + y).is_multiple_of(2) {
                    fb.set_pixel(x as i32, y as i32, 0xFF000000);
                }
            }
        }

        let original_pixel = fb.get_pixel(width as i32 - 1, height as i32 - 1).unwrap();

        // Apply DoF
        // Focus at 1.0, range 1.0. Right half should blur.
        let config = DepthOfFieldConfig {
            focus_dist: 1.0,
            focus_range: 1.0,
            blur_radius: 2,
        };
        apply_depth_of_field(&mut fb, &zb, &config);

        // Check if out-of-focus pixel changed
        let new_pixel = fb.get_pixel(width as i32 - 1, height as i32 - 1).unwrap();

        assert_ne!(
            original_pixel, new_pixel,
            "Out of focus pixel should be modified by blur"
        );

        // Check if in-focus pixel is UNCHANGED (or minimally changed)
        let focus_pixel_orig = 0xFF000000; // (0,0) is black
        let focus_pixel_new = fb.get_pixel(0, 0).unwrap();
        // Since factor should be 0.0 for dist=0, it should be exact.
        assert_eq!(
            focus_pixel_orig, focus_pixel_new,
            "In focus pixel should not change"
        );
    }
}
