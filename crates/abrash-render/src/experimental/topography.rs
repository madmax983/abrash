//! Topographical/Waveform Filter (Joy Division Style)
//!
//! A post-processing effect that converts the image into a series of horizontal
//! waveforms based on pixel luminance, simulating a 3D topographical map or
//! the classic Joy Division 'Unknown Pleasures' album cover.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::pixel_luminance;
use std::cell::RefCell;

/// Configuration for the Topography filter.
#[derive(Debug, Clone, Copy)]
pub struct TopographyConfig {
    /// Distance between each horizontal scanline (in pixels).
    pub line_spacing: usize,
    /// Maximum upward displacement for bright pixels (in pixels).
    pub amplitude: f32,
    /// Color of the waveform lines (ARGB).
    pub line_color: u32,
    /// Background color to fill beneath the waveforms (ARGB).
    pub background_color: u32,
}

impl Default for TopographyConfig {
    fn default() -> Self {
        Self {
            line_spacing: 8,
            amplitude: 25.0,
            line_color: 0xFF_FF_FF_FF,       // White lines
            background_color: 0xFF_00_00_00, // Black background
        }
    }
}

/// Applies the Topography waveform effect to the framebuffer.
pub fn apply_topography(fb: &mut Framebuffer, config: &TopographyConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || config.line_spacing == 0 {
        return;
    }

    // Optimization: avoid allocating a copy of the framebuffer per frame.
    // We only need the source pixels to sample luminance.
    thread_local! {
        static SRC_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    }

    SRC_BUFFER.with(|src_buf| {
        let mut src = src_buf.borrow_mut();
        src.resize(fb.as_slice().len(), 0);
        src.copy_from_slice(fb.as_slice());

        // Clear the target framebuffer to background color first
        fb.clear(config.background_color);

        let slice = fb.as_mut_slice();

        // Iterate top-to-bottom (back-to-front painter's algorithm)
        for base_y in (0..height).step_by(config.line_spacing) {
            let mut prev_disp_y = base_y;

            for x in 0..width {
                // Sample luminance from the source buffer
                let pixel = src[(base_y * width) + x];
                let luma = f32::from(pixel_luminance(pixel)) / 255.0;

                // Displacement goes upwards (subtract from Y)
                // We square the luma to make the peaks slightly sharper
                let displacement = luma * luma * config.amplitude;

                let mut disp_y = base_y as isize - displacement as isize;
                // Clamp to screen bounds
                if disp_y < 0 {
                    disp_y = 0;
                }
                let disp_y = disp_y as usize;

                if x > 0 {
                    let y_min = prev_disp_y.min(disp_y);
                    let y_max = prev_disp_y.max(disp_y);

                    // Draw line connecting previous Y to current Y
                    for y in y_min..=y_max {
                        if y < height {
                            slice[(y * width) + x] = config.line_color;
                        }
                    }

                    // Fill background below the lowest part of the line down to base_y
                    for y in (y_max + 1)..=base_y {
                        if y < height {
                            slice[(y * width) + x] = config.background_color;
                        }
                    }
                } else {
                    // First pixel in the row
                    if disp_y < height {
                        slice[(disp_y * width) + x] = config.line_color;
                    }
                    for y in (disp_y + 1)..=base_y {
                        if y < height {
                            slice[(y * width) + x] = config.background_color;
                        }
                    }
                }

                prev_disp_y = disp_y;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_topography() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Draw a bright square in the middle
        for y in 40..60 {
            for x in 40..60 {
                fb.set_pixel(x as i32, y as i32, 0xFFFF_FFFF);
            }
        }

        let config = TopographyConfig::default();
        apply_topography(&mut fb, &config);

        // Assert some pixel is white (line color)
        let mut has_white = false;
        let mut has_black = false;
        for y in 0..100 {
            for x in 0..100 {
                let p = fb.get_pixel(x, y).unwrap();
                if p == config.line_color {
                    has_white = true;
                } else if p == config.background_color {
                    has_black = true;
                }
            }
        }
        assert!(has_white, "Should have waveform lines");
        assert!(has_black, "Should have background color");
    }
}
