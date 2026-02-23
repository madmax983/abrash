use crate::framebuffer::Framebuffer;
use crate::post_process::blur::{box_blur_horizontal, box_blur_vertical};
use std::cell::RefCell;

// Reusable scratch buffers for Bloom
// Allocating these per-frame would be slow.
// We use a thread-local RefCell to reuse them safely.
#[derive(Default)]
struct BloomContext {
    bright_pixels: Vec<u32>,
    blurred_buffer: Vec<u32>,
    // Intermediate buffer for separable blur
    // Stored as i32 for accumulation precision? No, box blur usually uses u32 or f32.
    // The current blur implementation expects &mut [u32].
    acc_buffer: Vec<i32>,
}

thread_local! {
    static BLOOM_CONTEXT: RefCell<BloomContext> = RefCell::new(BloomContext::default());
}

/// Applies a Bloom effect to the framebuffer.
///
/// 1. Threshold: Extract bright pixels.
/// 2. Blur: Apply a box blur to the bright pixels.
/// 3. Composite: Add the blurred result back to the original image.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `threshold` - Brightness threshold (0-255). Pixels brighter than this contribute to bloom.
/// * `blur_radius` - Radius of the box blur.
/// * `intensity` - Multiplier for the bloom effect.
pub fn apply_bloom(fb: &mut Framebuffer, threshold: u8, blur_radius: usize, intensity: f32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let len = width * height;

    BLOOM_CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();

        // 1. Resize buffers if needed
        if ctx.bright_pixels.len() != len {
            ctx.bright_pixels.resize(len, 0);
            ctx.blurred_buffer.resize(len, 0);
            ctx.acc_buffer.resize(len, 0); // Not used by current simple blur, but ready
        }

        let pixels = fb.as_slice();
        let BloomContext {
            bright_pixels,
            blurred_buffer: blurred,
            acc_buffer: acc,
        } = &mut *ctx;

        // 2. Extract Bright Pixels
        for (i, &p) in pixels.iter().enumerate() {
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;
            // Simple luminance or max component
            let max_c = r.max(g).max(b) as u8;

            if max_c > threshold {
                bright_pixels[i] = p;
            } else {
                bright_pixels[i] = 0xFF000000; // Black
            }
        }

        // 3. Blur (Separable Box Blur)
        // Horizontal pass: bright_pixels -> blurred
        box_blur_horizontal(
            bright_pixels,
            blurred,
            width,
            height,
            blur_radius as u32,
        );
        // Vertical pass: blurred -> bright_pixels (ping-pong)
        // We reuse bright_pixels as the destination for the second pass
        box_blur_vertical(
            blurred,
            bright_pixels,
            acc,
            width,
            height,
            blur_radius as u32,
        );

        // 4. Composite (Additive blending)
        // Final result is in bright_pixels
        let pixels_mut = fb.as_mut_slice();
        for (dest, &src) in pixels_mut.iter_mut().zip(bright_pixels.iter()) {
            let r_src = ((src >> 16) & 0xFF) as f32;
            let g_src = ((src >> 8) & 0xFF) as f32;
            let b_src = (src & 0xFF) as f32;

            let r_dest = ((*dest >> 16) & 0xFF) as f32;
            let g_dest = ((*dest >> 8) & 0xFF) as f32;
            let b_dest = (*dest & 0xFF) as f32;

            let r_final = (r_dest + r_src * intensity).min(255.0) as u32;
            let g_final = (g_dest + g_src * intensity).min(255.0) as u32;
            let b_final = (b_dest + b_src * intensity).min(255.0) as u32;

            *dest = 0xFF000000 | (r_final << 16) | (g_final << 8) | b_final;
        }
    });
}
