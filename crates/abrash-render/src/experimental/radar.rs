//! Radar Scope Filter
//!
//! Simulates a radar screen. It reads the 3D Z-Buffer to detect geometry
//! and draws a sweeping radar line and fading blips.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use std::cell::RefCell;
use std::f32::consts::PI;

/// Configuration for the Radar effect.
#[derive(Debug, Clone, Copy)]
pub struct RadarConfig {
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
    pub sweep_speed: f32,
    pub grid_color: u32,
    pub sweep_color: u32,
    pub blip_color: u32,
    pub background_color: u32,
    pub blip_fade_rate: u8,
}

impl Default for RadarConfig {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            radius: 0.45,
            sweep_speed: 2.0,
            grid_color: 0xFF_00_88_00,
            sweep_color: 0xFF_00_FF_00,
            blip_color: 0xFF_AA_FF_AA,
            background_color: 0xFF_00_22_00,
            blip_fade_rate: 5,
        }
    }
}

/// The state of the radar simulation.
pub struct RadarState {
    pub angle: f32,
    pub blip_buffer: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

impl Default for RadarState {
    fn default() -> Self {
        Self {
            angle: 0.0,
            blip_buffer: Vec::new(),
            width: 0,
            height: 0,
        }
    }
}

thread_local! {
    static STATE: RefCell<RadarState> = RefCell::new(RadarState::default());
}

#[inline(always)]
fn alpha_blend(src: u32, dst: u32, alpha_factor: f32) -> u32 {
    let alpha = (alpha_factor * 255.0).clamp(0.0, 255.0) as u32;
    if alpha == 0 {
        return dst;
    }
    if alpha == 255 {
        return src;
    }
    let inv_alpha = 255 - alpha;

    let src_rb = src & 0x00FF_00FF;
    let src_g = (src >> 8) & 0x00FF_00FF;
    let dst_rb = dst & 0x00FF_00FF;
    let dst_g = (dst >> 8) & 0x00FF_00FF;

    let rb = ((src_rb * alpha + dst_rb * inv_alpha) >> 8) & 0x00FF_00FF;
    let g = ((src_g * alpha + dst_g * inv_alpha) >> 8) & 0x00FF_00FF;

    (0xFF << 24) | (g << 8) | rb
}

#[inline(always)]
fn distance_sq(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    dx * dx + dy * dy
}

/// Applies a radar scope post-processing effect.
pub fn apply_radar(fb: &mut Framebuffer, zb: &ZBuffer, config: &RadarConfig) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let cx_pixel = config.center_x * width as f32;
    let cy_pixel = config.center_y * height as f32;
    let max_dimension = if width < height { width } else { height } as f32;
    let radius_pixel = config.radius * max_dimension;
    let radius_sq = radius_pixel * radius_pixel;

    STATE.with(|state_ref| {
        let mut state = state_ref.borrow_mut();

        // Resize blip buffer if dimensions changed
        if state.width != width || state.height != height {
            state.blip_buffer = vec![0; width * height];
            state.width = width;
            state.height = height;
        }

        // Advance sweep angle
        state.angle += config.sweep_speed * 0.016; // Approx delta time
        if state.angle > PI * 2.0 {
            state.angle -= PI * 2.0;
        }

        let sweep_angle = state.angle;
        let pixels = fb.as_mut_slice();
        let depths = zb.as_slice();

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let dx = x as f32 - cx_pixel;
                let dy = y as f32 - cy_pixel;
                let dist_sq = dx * dx + dy * dy;

                // Inside the radar radius
                if dist_sq <= radius_sq {
                    let mut pixel_angle = dy.atan2(dx);
                    if pixel_angle < 0.0 {
                        pixel_angle += PI * 2.0;
                    }

                    // Calculate angular difference
                    let mut angle_diff = sweep_angle - pixel_angle;
                    if angle_diff < 0.0 {
                        angle_diff += PI * 2.0;
                    }

                    // Draw background & grid
                    let dist = dist_sq.sqrt();
                    let is_grid =
                        (dist % (radius_pixel / 4.0)) < 1.0 || (dx.abs() < 1.0) || (dy.abs() < 1.0);

                    let base_color = if is_grid {
                        config.grid_color
                    } else {
                        config.background_color
                    };

                    // Sweep line and trail
                    let sweep_intensity = if angle_diff < 0.1 {
                        1.0 // Bright line
                    } else if angle_diff < PI {
                        (1.0 - (angle_diff / PI)).powf(2.0) // Fading trail
                    } else {
                        0.0
                    };

                    // Detect blips
                    if angle_diff < 0.1 && depths[idx] < 100.0 {
                        // Geometry threshold
                        state.blip_buffer[idx] = 255; // Max intensity
                    } else {
                        // Fade blips
                        let fade = config.blip_fade_rate;
                        state.blip_buffer[idx] = state.blip_buffer[idx].saturating_sub(fade);
                    }

                    let blip_intensity = f32::from(state.blip_buffer[idx]) / 255.0;

                    let mut final_color = base_color;
                    if sweep_intensity > 0.0 {
                        final_color = alpha_blend(config.sweep_color, final_color, sweep_intensity);
                    }
                    if blip_intensity > 0.0 {
                        final_color = alpha_blend(config.blip_color, final_color, blip_intensity);
                    }

                    // Mask exact boundary
                    if (dist - radius_pixel).abs() < 2.0 {
                        pixels[idx] = config.grid_color;
                    } else {
                        pixels[idx] = final_color;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radar_blips() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut zb = ZBuffer::new(10, 10).unwrap();

        // Put fake geometry
        zb.as_mut_slice()[55] = 10.0;

        let config = RadarConfig::default();

        // Clear state
        STATE.with(|state| {
            state.replace(RadarState::default());
        });

        apply_radar(&mut fb, &zb, &config);

        STATE.with(|state| {
            let st = state.borrow();
            assert_eq!(st.width, 10);
            assert_eq!(st.height, 10);
            assert_eq!(st.blip_buffer.len(), 100);
        });
    }
}
