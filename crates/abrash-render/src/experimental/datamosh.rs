//! Datamoshing / I-Frame failure simulation.
//!
//! A post-processing effect that simulates video compression artifacts, specifically
//! "datamoshing," where motion vectors from a new frame are incorrectly applied to
//! the pixel data of a previous frame (simulating a missing I-frame).

use crate::framebuffer::Framebuffer;

/// Configuration for the Datamosh effect.
#[derive(Debug, Clone, Copy)]
pub struct DatamoshConfig {
    /// Intensity of the moshing effect (how strongly motion vectors are applied).
    /// 0.0 means no effect (passthrough).
    pub intensity: f32,
    /// Threshold of luminance difference required to register as "motion".
    pub threshold: f32,
    /// If true, a new "I-frame" is forced, clearing the mosh state and
    /// drawing the current frame perfectly.
    pub force_iframe: bool,
}

impl Default for DatamoshConfig {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            threshold: 10.0,
            force_iframe: false,
        }
    }
}

pub struct DatamoshFilter {
    prev_moshed_frame: Vec<u32>,
    prev_clean_frame: Vec<u32>,
}

impl Default for DatamoshFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl DatamoshFilter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            prev_moshed_frame: Vec::new(),
            prev_clean_frame: Vec::new(),
        }
    }

    pub fn apply(&mut self, fb: &mut Framebuffer, config: &DatamoshConfig) {
        if config.intensity <= 0.0 {
            return;
        }

        let width = fb.width() as usize;
        let height = fb.height() as usize;

        if width == 0 || height == 0 {
            return;
        }

        let size = width * height;

        // Resize buffers if dimensions changed or on first run
        if self.prev_moshed_frame.len() != size || config.force_iframe {
            self.prev_moshed_frame.clear();
            self.prev_moshed_frame.resize(size, 0);
            self.prev_moshed_frame.copy_from_slice(fb.as_slice());

            self.prev_clean_frame.clear();
            self.prev_clean_frame.resize(size, 0);
            self.prev_clean_frame.copy_from_slice(fb.as_slice());
            return;
        }

        let pixels = fb.as_mut_slice();

        // Create a temporary buffer to hold the newly moshed frame
        let mut new_moshed = vec![0u32; size];
        new_moshed.copy_from_slice(self.prev_moshed_frame.as_slice());

        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let idx = y * width + x;

                let curr_p = pixels[idx];
                let clean_p = self.prev_clean_frame[idx];

                // Luminance calculation
                let get_lum = |p: u32| -> f32 {
                    let r = ((p >> 16) & 0xFF) as f32;
                    let g = ((p >> 8) & 0xFF) as f32;
                    let b = (p & 0xFF) as f32;
                    0.299 * r + 0.587 * g + 0.114 * b
                };

                let curr_lum = get_lum(curr_p);
                let clean_lum = get_lum(clean_p);
                let diff = (curr_lum - clean_lum).abs();

                // If change exceeds threshold, we consider it "motion" and try to find a vector
                if diff > config.threshold {
                    // Naive block matching: find the most similar pixel in the clean frame's neighborhood
                    let mut best_dx = 0;
                    let mut best_dy = 0;
                    let mut min_diff = diff;

                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }

                            let search_idx = ((y as i32 + dy) as usize) * width + ((x as i32 + dx) as usize);
                            let search_lum = get_lum(self.prev_clean_frame[search_idx]);
                            let search_diff = (curr_lum - search_lum).abs();

                            if search_diff < min_diff {
                                min_diff = search_diff;
                                best_dx = dx;
                                best_dy = dy;
                            }
                        }
                    }

                    // Apply the motion vector scaled by intensity to the MOSHED frame
                    if best_dx != 0 || best_dy != 0 {
                        let mosh_dx = (best_dx as f32 * config.intensity).round() as i32;
                        let mosh_dy = (best_dy as f32 * config.intensity).round() as i32;

                        let src_x = (x as i32 - mosh_dx).clamp(0, width as i32 - 1) as usize;
                        let src_y = (y as i32 - mosh_dy).clamp(0, height as i32 - 1) as usize;

                        let src_idx = src_y * width + src_x;
                        new_moshed[idx] = self.prev_moshed_frame[src_idx];
                    } else {
                        new_moshed[idx] = self.prev_moshed_frame[idx];
                    }
                } else {
                    new_moshed[idx] = self.prev_moshed_frame[idx];
                }
            }
        }

        // Save the clean frame for the NEXT iteration's diff
        self.prev_clean_frame.copy_from_slice(pixels);

        // Write moshed result to screen and update history
        pixels.copy_from_slice(&new_moshed);
        self.prev_moshed_frame.copy_from_slice(&new_moshed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_datamosh() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Frame 1: Solid black
        fb.clear(0xFF00_0000);
        let config_iframe = DatamoshConfig {
            force_iframe: true,
            ..Default::default()
        };
        let mut filter = DatamoshFilter::new();
        filter.apply(&mut fb, &config_iframe);

        // Frame 2: Draw a white square in the top left
        fb.clear(0xFF00_0000);
        for y in 0..5 {
            for x in 0..5 {
                fb.set_pixel(x, y, 0xFFFF_FFFF);
            }
        }
        let config_mosh = DatamoshConfig {
            intensity: 1.0,
            force_iframe: false,
            ..Default::default()
        };

        let mut fb_clone = Framebuffer::new(width, height).unwrap();
        fb_clone.as_mut_slice().copy_from_slice(fb.as_slice());

        filter.apply(&mut fb, &config_mosh);

        // Assert that the buffer was actually changed by the datamosh logic
        // (i.e. motion vectors smeared the pixels)
        let mut different = false;
        for i in 0..(width * height) as usize {
            if fb.as_slice()[i] != fb_clone.as_slice()[i] {
                different = true;
                break;
            }
        }
        assert!(different, "Datamosh filter did not modify the framebuffer");
    }
}
