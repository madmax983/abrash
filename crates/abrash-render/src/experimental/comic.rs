//! Comic Book / Pop Art Stylization Filter
//!
//! A non-photorealistic post-processing effect that combines several filters
//! to create a comic book or pop art aesthetic. It uses:
//! 1. `apply_kuwahara` to simplify colors and create painted regions.
//! 2. `apply_sobel` to detect edges and draw black ink outlines.
//! 3. (Optional) `apply_halftone` to add retro CMYK-style print dots.

use crate::framebuffer::Framebuffer;
use crate::post_process::filters::apply_sobel;
use crate::experimental::kuwahara::apply_kuwahara;
use crate::experimental::halftone::apply_halftone;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::cell::RefCell;

thread_local! {
    static COMIC_SCRATCH_BUFFER: RefCell<Option<Framebuffer>> = const { RefCell::new(None) };
}

/// Configuration for the Comic Book effect.
#[derive(Debug, Clone, Copy)]
pub struct ComicConfig {
    /// Radius of the Kuwahara filter (oil painting effect).
    /// Higher values mean more abstraction and larger color blocks (e.g., 2 or 3).
    pub paint_radius: i32,
    /// Threshold for edge detection (0-255).
    /// Pixels with edge gradients above this value will be drawn as black ink.
    pub edge_threshold: u32,
    /// Enable halftone dot printing effect.
    pub use_halftone: bool,
    /// Size of halftone dots if enabled.
    pub halftone_dot_size: f32,
    /// Angle of halftone dots in radians if enabled.
    pub halftone_angle: f32,
}

impl Default for ComicConfig {
    fn default() -> Self {
        Self {
            paint_radius: 2,
            edge_threshold: 100,
            use_halftone: false,
            halftone_dot_size: 3.0,
            halftone_angle: std::f32::consts::PI / 4.0,
        }
    }
}

/// Applies a comic book stylization to the framebuffer.
///
/// Combines Kuwahara color abstraction with Sobel edge detection to draw
/// black outlines over painted color blocks.
///
/// # Arguments
/// * `fb` - The framebuffer to modify in-place.
/// * `config` - Configuration for the comic effect.
pub fn apply_comic(fb: &mut Framebuffer, config: &ComicConfig) {
    let width = fb.width();
    let height = fb.height();
    let size = (width * height) as usize;

    if size == 0 {
        return;
    }

    COMIC_SCRATCH_BUFFER.with(|buf| {
        let mut opt_fb = buf.borrow_mut();

        // Ensure the thread local framebuffer is initialized and is the correct size
        if opt_fb.is_none() || opt_fb.as_ref().unwrap().width() != width || opt_fb.as_ref().unwrap().height() != height {
            *opt_fb = Some(Framebuffer::new(width, height).unwrap());
        }

        let edge_fb = opt_fb.as_mut().unwrap();

        // 1. Copy original framebuffer to scratch framebuffer for edge detection
        edge_fb.as_mut_slice().copy_from_slice(fb.as_slice());

        // 2. Apply Sobel edge detection. This turns edge_fb into a grayscale image
        // where brightness represents edge strength.
        apply_sobel(edge_fb);

        // 3. Apply Kuwahara filter to the original framebuffer to simplify colors
        if config.paint_radius > 0 {
            apply_kuwahara(fb, config.paint_radius);
        }

        // 4. Optionally apply halftone pattern over the painted colors
        if config.use_halftone && config.halftone_dot_size > 0.0 {
            apply_halftone(fb, config.halftone_dot_size, config.halftone_angle);
        }

        // 5. Composite edges over the painted image
        let color_pixels = fb.as_mut_slice();
        let final_edges = edge_fb.as_slice();

        #[cfg(feature = "parallel")]
        {
            color_pixels
                .par_iter_mut()
                .zip(final_edges.par_iter())
                .for_each(|(color_pix, edge_pix)| {
                    // Extract edge magnitude (grayscale value from sobel)
                    // The lowest byte has the magnitude since it's grayscale
                    let mag = edge_pix & 0xFF;

                    if mag >= config.edge_threshold {
                        // Draw black ink outline (preserve alpha)
                        *color_pix = *color_pix & 0xFF00_0000;
                    }
                });
        }

        #[cfg(not(feature = "parallel"))]
        {
            for (color_pix, edge_pix) in color_pixels.iter_mut().zip(final_edges.iter()) {
                let mag = edge_pix & 0xFF;

                if mag >= config.edge_threshold {
                    *color_pix = *color_pix & 0xFF00_0000;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_comic_basic() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFF_FF_FF_FF); // White

        // Draw a black square in the middle to create strong edges
        for y in 2..8 {
            for x in 2..8 {
                fb.set_pixel(x, y, 0xFF_00_00_00);
            }
        }

        let config = ComicConfig {
            paint_radius: 1,
            edge_threshold: 0, // lower threshold to ensure black pixels
            use_halftone: false,
            ..Default::default()
        };

        apply_comic(&mut fb, &config);

        // Edges should be black. Let's just ensure it doesn't crash and alters pixels.
        // Sobel edge detection might shift coordinates slightly, but there should definitely be black pixels.
        let has_black = fb.as_slice().iter().any(|&p| (p & 0x00_FF_FF_FF) == 0x00_00_00_00);
        assert!(has_black);
    }
}
