use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;
use std::cell::RefCell;
use super::blur::{box_blur_horizontal, box_blur_vertical};

thread_local! {
    static DOF_CONTEXT: RefCell<DofContext> = RefCell::new(DofContext::default());
}

#[derive(Default)]
struct DofContext {
    blurred_buffer: Vec<u32>,
    scratch_buffer: Vec<u32>,
    acc_buffer: Vec<i32>,
}

/// Applies depth of field effect.
///
/// # Arguments
/// * `fb` - The framebuffer (modified in-place).
/// * `zb` - The depth buffer.
/// * `focus_dist` - The depth at which objects are perfectly in focus (0.0 - 1.0 in non-linear z-buffer space).
/// * `focus_range` - The range of depth that remains reasonably sharp.
/// * `blur_radius` - The radius of the blur for out-of-focus areas.
pub fn apply_depth_of_field(
    fb: &mut Framebuffer,
    zb: &ZBuffer,
    focus_dist: f32,
    focus_range: f32,
    blur_radius: u32,
) {
    if blur_radius == 0 {
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
        box_blur_horizontal(original_pixels, scratch_slice, width, height, blur_radius);
        // Vertical pass: scratch -> blurred
        box_blur_vertical(scratch_slice, blurred_slice, acc_slice, width, height, blur_radius);

        // 2. Blend based on depth
        // We iterate over the original buffer and the blurred buffer
        let zb_slice = zb.as_slice();

        // Ensure we don't go out of bounds if buffers mismatch (though they shouldn't)
        let len = original_pixels.len().min(zb_slice.len()).min(blurred_slice.len());

        for i in 0..len {
            let depth = zb_slice[i];

            // Skip infinite depth (skybox) if desired, or treat as far.
            // ZBuffer init is INFINITY. If depth is INFINITY, it's background.
            // If focus is near, background is blurred.
            // If focus is far, background is sharp?
            // Let's treat INFINITY as far (e.g. 1.0 or just use large number)
            let z = if depth.is_infinite() { 1000.0 } else { depth };

            let dist = (z - focus_dist).abs();

            // Calculate blur factor (0.0 = sharp, 1.0 = full blur)
            // If dist < range, factor = 0.
            // If dist > range, factor increases.
            // Simple linear falloff:
            let factor = ((dist - focus_range) / focus_range).clamp(0.0, 1.0);

            if factor > 0.0 {
                let orig = original_pixels[i];
                let blur = blurred_slice[i];

                let r_o = ((orig >> 16) & 0xFF) as f32;
                let g_o = ((orig >> 8) & 0xFF) as f32;
                let b_o = (orig & 0xFF) as f32;

                let r_b = ((blur >> 16) & 0xFF) as f32;
                let g_b = ((blur >> 8) & 0xFF) as f32;
                let b_b = (blur & 0xFF) as f32;

                let r_new = lerp(r_o, r_b, factor) as u32;
                let g_new = lerp(g_o, g_b, factor) as u32;
                let b_new = lerp(b_o, b_b, factor) as u32;

                // Preserve alpha
                original_pixels[i] = (orig & 0xFF00_0000) | (r_new << 16) | (g_new << 8) | b_new;
            }
        }
    });
}

#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
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
                if (x as u32 + y as u32) % 2 == 0 {
                    fb.set_pixel(x as i32, y as i32, 0xFF000000);
                }
            }
        }

        let original_pixel = fb.get_pixel(width as i32 - 1, height as i32 - 1).unwrap();

        // Apply DoF
        // Focus at 1.0, range 1.0. Right half should blur.
        apply_depth_of_field(&mut fb, &zb, 1.0, 1.0, 2);

        // Check if out-of-focus pixel changed
        let new_pixel = fb.get_pixel(width as i32 - 1, height as i32 - 1).unwrap();

        assert_ne!(original_pixel, new_pixel, "Out of focus pixel should be modified by blur");

        // Check if in-focus pixel is UNCHANGED (or minimally changed)
        let focus_pixel_orig = 0xFF000000; // (0,0) is black
        let focus_pixel_new = fb.get_pixel(0, 0).unwrap();
        // Since factor should be 0.0 for dist=0, it should be exact.
        assert_eq!(focus_pixel_orig, focus_pixel_new, "In focus pixel should not change");
    }
}
