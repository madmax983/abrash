//! Vision post-processing effects.
//!
//! Provides advanced visualization modes like Night Vision, Thermal Imaging, and Sonar.
//! These effects modify the framebuffer based on color and depth information.

use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;

/// Available vision modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionMode {
    /// Green-tinted night vision with noise and vignette.
    Night,
    /// Heat map visualization based on depth.
    Thermal,
    /// Scanning pulse effect based on depth.
    Sonar,
}

/// Configuration for the vision effect.
#[derive(Debug, Clone, Copy)]
pub struct VisionConfig {
    /// The active vision mode.
    pub mode: VisionMode,
    /// Current time in seconds (used for Sonar pulse).
    pub time: f32,
    /// Intensity of the effect (0.0 to 1.0).
    pub intensity: f32,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            mode: VisionMode::Night,
            time: 0.0,
            intensity: 1.0,
        }
    }
}

/// A simple pseudo-random number generator using Xorshift algorithm.
/// Used for generating noise without external dependencies.
struct XorShift {
    state: u32,
}

impl XorShift {
    const fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    const fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Returns a float in [0.0, 1.0).
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }
}

/// Applies the selected vision effect to the framebuffer.
pub fn apply_vision(fb: &mut Framebuffer, zb: &ZBuffer, config: &VisionConfig) {
    match config.mode {
        VisionMode::Night => apply_night_vision(fb, config),
        VisionMode::Thermal => apply_thermal_vision(fb, zb, config),
        VisionMode::Sonar => apply_sonar_vision(fb, zb, config),
    }
}

fn apply_night_vision(fb: &mut Framebuffer, config: &VisionConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();
    let mut rng = XorShift::new((config.time * 1000.0) as u32);

    let center_x = width as f32 * 0.5;
    let center_y = height as f32 * 0.5;
    #[allow(clippy::imprecise_flops)]
    let max_radius = (center_x * center_x + center_y * center_y).sqrt();

    for y in 0..height {
        let dy = y as f32 - center_y;
        for x in 0..width {
            let dx = x as f32 - center_x;
            #[allow(clippy::imprecise_flops)]
            let dist = (dx * dx + dy * dy).sqrt();

            // Vignette: Darken edges
            let vignette = (1.0 - (dist / max_radius).powi(2)).max(0.0);

            let idx = y * width + x;
            let pixel = pixels[idx];

            // Extract RGB
            let r = ((pixel >> 16) & 0xFF) as f32;
            let g = ((pixel >> 8) & 0xFF) as f32;
            let b = (pixel & 0xFF) as f32;

            // Luminance (Standard Rec. 709)
            let lum = r * 0.2126 + g * 0.7152 + b * 0.0722;

            // Add Noise
            let noise = (rng.next_f32() - 0.5) * 50.0 * config.intensity;

            // Map to Green phosphor
            let val = (lum + noise) * vignette * config.intensity;
            let val_clamped = val.clamp(0.0, 255.0) as u32;

            // Result is mostly green, slight blue/red tint for phosphor feel
            pixels[idx] =
                0xFF00_0000 | ((val_clamped / 5) << 16) | (val_clamped << 8) | (val_clamped / 5);
        }
    }
}

fn apply_thermal_vision(fb: &mut Framebuffer, zb: &ZBuffer, _config: &VisionConfig) {
    let _width = fb.width() as usize;
    let _height = fb.height() as usize;
    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    for (i, &depth) in depths.iter().enumerate() {
        if depth.is_infinite() {
            pixels[i] = 0xFF00_0000; // Background is black (Coldest)
            continue;
        }

        // Depth in NDC is typically [-1.0, 1.0].
        // -1.0 is Near (Hot), 1.0 is Far (Cold).
        // Map [-1.0, 1.0] -> [0.0, 1.0] for the gradient.
        // t = 0.0 (Hot), t = 1.0 (Cold).
        // t = (depth - (-1.0)) / 2.0 = (depth + 1.0) / 2.0.
        // Invert for Hot->Cold mapping: 1.0 - t

        let t = ((depth + 1.0) * 0.5).clamp(0.0, 1.0);
        let heat = 1.0 - t;

        pixels[i] = get_thermal_color(heat);
    }
}

fn get_thermal_color(t: f32) -> u32 {
    // t: 0.0 (Cold) -> 1.0 (Hot)
    // 0.0 - 0.2: Black -> Blue
    // 0.2 - 0.5: Blue -> Purple
    // 0.5 - 0.8: Purple -> Red
    // 0.8 - 1.0: Red -> Yellow -> White

    let (r, g, b) = if t < 0.2 {
        // Black to Blue
        let local_t = t / 0.2;
        (0.0, 0.0, local_t)
    } else if t < 0.5 {
        // Blue to Purple (Blue + Red)
        let local_t = (t - 0.2) / 0.3;
        (local_t, 0.0, 1.0) // B=1, R goes 0->1
    } else if t < 0.8 {
        // Purple to Red
        let local_t = (t - 0.5) / 0.3;
        (1.0, 0.0, 1.0 - local_t) // R=1, B goes 1->0
    } else {
        // Red to Yellow (add Green) to White (add Blue)
        let local_t = (t - 0.8) / 0.2;
        if local_t < 0.5 {
            // Red to Yellow
            (1.0, local_t * 2.0, 0.0)
        } else {
            // Yellow to White
            (1.0, 1.0, (local_t - 0.5) * 2.0)
        }
    };

    let r_byte = (r * 255.0) as u32;
    let g_byte = (g * 255.0) as u32;
    let b_byte = (b * 255.0) as u32;

    0xFF00_0000 | (r_byte << 16) | (g_byte << 8) | b_byte
}

fn apply_sonar_vision(fb: &mut Framebuffer, zb: &ZBuffer, config: &VisionConfig) {
    let _width = fb.width() as usize;
    let _height = fb.height() as usize;
    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    // Pulse moves from -1.0 to 1.0 over time.
    // Period = 2.0 seconds?
    let period = 2.0;
    let phase = (config.time % period) / period; // 0.0 to 1.0
    let pulse_pos = -1.0 + phase * 2.0; // -1.0 to 1.0
    let thickness = 0.05;

    for (i, &depth) in depths.iter().enumerate() {
        if depth.is_infinite() {
            pixels[i] = 0xFF00_0010; // Very dim blue background
            continue;
        }

        let dist = (depth - pulse_pos).abs();

        if dist < thickness {
            // High intensity at pulse
            let intensity = 1.0 - (dist / thickness);
            let val = (intensity * 255.0) as u32;
            pixels[i] = 0xFF00_0000 | (val << 8) | val; // Cyan/Greenish
        } else {
            // Dim outline of objects
            // Edge detection logic is expensive here, so just dim the original color
            // or use a flat "wireframe" color based on depth derivative?
            // Let's just use a dark blue base.
            let base = ((depth + 1.0) * 0.5 * 50.0) as u32;
            pixels[i] = 0xFF00_0000 | (base << 8) | (base + 20);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_vision_modifies_buffer() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Fill with some data
        fb.clear(0xFFFF_FFFF); // White
        zb.clear();
        zb.test_and_set(5, 5, 0.0); // Center pixel at 0.0 depth

        let config = VisionConfig {
            mode: VisionMode::Night,
            time: 0.0,
            intensity: 1.0,
        };

        // Before: Center is White
        assert_eq!(fb.get_pixel(5, 5), Some(0xFFFF_FFFF));

        apply_vision(&mut fb, &zb, &config);

        // After: Center should be Green-ish (Night Vision)
        let pixel = fb.get_pixel(5, 5).unwrap();
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        assert!(g > r, "Green component should be dominant");
        assert!(g > b, "Green component should be dominant");
    }

    #[test]
    fn test_thermal_vision() {
        let width = 2;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Near (-1.0) -> Hot -> White/Yellow
        zb.test_and_set(0, 0, -1.0);
        // Mid-Far (0.5) -> Cooler -> Blueish
        zb.test_and_set(1, 0, 0.5);

        let config = VisionConfig {
            mode: VisionMode::Thermal,
            ..Default::default()
        };

        apply_vision(&mut fb, &zb, &config);

        let hot_pixel = fb.get_pixel(0, 0).unwrap();
        let cold_pixel = fb.get_pixel(1, 0).unwrap();

        // Check Hot (should be bright/red/yellow)
        let hot_r = (hot_pixel >> 16) & 0xFF;
        assert!(hot_r > 200, "Hot pixel should have high Red");

        // Check Cold (should be blue dominant)
        // depth=0.5 -> t=0.75 -> heat=0.25
        // get_thermal_color(0.25): Blue=1.0, Red=0.16.
        let cold_b = cold_pixel & 0xFF;
        let cold_r = (cold_pixel >> 16) & 0xFF;
        assert!(cold_b > cold_r, "Cold pixel should be Blue-dominant");
    }

    #[test]
    fn test_sonar_pulse() {
        let width = 2;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // Pixel at 0.0
        zb.test_and_set(0, 0, 0.0);

        // Pulse at 0.0 (time = 1.0, period = 2.0 -> phase = 0.5 -> pos = 0.0)
        let config = VisionConfig {
            mode: VisionMode::Sonar,
            time: 1.0,
            intensity: 1.0,
        };

        apply_vision(&mut fb, &zb, &config);

        let hit_pixel = fb.get_pixel(0, 0).unwrap();
        // Should be bright cyan/green
        let g = (hit_pixel >> 8) & 0xFF;
        let b = hit_pixel & 0xFF;
        assert!(g > 100, "Pulse hit should be bright");
        assert!(b > 100, "Pulse hit should be bright");
    }
}
