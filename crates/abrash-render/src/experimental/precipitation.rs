//! Depth-Aware Precipitation Filter
//!
//! Simulates 2D rain or snow falling over the screen that interacts with the 3D Z-Buffer.
//! Raindrops are assigned a random depth (Z). If they fall behind a surface in the Z-Buffer,
//! they "hit" the surface and create a splash.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;
use abrash_core::zbuffer::ZBuffer;
use std::cell::RefCell;

/// The state of a single precipitation drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropState {
    Falling,
    Splashing,
}

/// A single drop of precipitation.
#[derive(Debug, Clone, Copy)]
pub struct Drop {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub state: DropState,
    pub life: f32, // Frames remaining for splash
    pub color: u32,
}

/// Configuration for the precipitation effect.
#[derive(Debug, Clone, Copy)]
pub struct PrecipitationConfig {
    pub max_drops: usize,
    pub drop_color: u32,
    pub splash_color: u32,
    pub gravity: f32,
    pub wind: f32,
    pub drop_speed_min: f32,
    pub drop_speed_max: f32,
    pub z_min: f32,
    pub z_max: f32,
    pub splash_duration: f32,
}

impl Default for PrecipitationConfig {
    fn default() -> Self {
        Self {
            max_drops: 2000,
            drop_color: 0x88_AA_CC_FF, // Semi-transparent light blue
            splash_color: 0xCC_EE_FF_FF,
            gravity: 0.2,
            wind: 1.0,
            drop_speed_min: 5.0,
            drop_speed_max: 15.0,
            z_min: 1.0,
            z_max: 50.0,
            splash_duration: 5.0,
        }
    }
}

/// The state of the precipitation simulation.
pub struct PrecipitationState {
    pub drops: Vec<Drop>,
    pub rng: XorShift32,
}

impl Default for PrecipitationState {
    fn default() -> Self {
        Self {
            drops: Vec::new(),
            rng: XorShift32::new(0x1234_5678),
        }
    }
}

thread_local! {
    static STATE: RefCell<PrecipitationState> = RefCell::new(PrecipitationState::default());
}

#[inline(always)]
const fn alpha_blend_pixel(src: u32, dst: u32) -> u32 {
    let alpha = (src >> 24) & 0xFF;
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
fn scale_alpha(color: u32, factor: f32) -> u32 {
    let mut a = ((color >> 24) as f32 * factor) as u32;
    if a > 255 {
        a = 255;
    }
    (a << 24) | (color & 0x00FF_FFFF)
}

/// Applies a depth-aware precipitation (rain/snow) effect over the framebuffer.
///
/// Raindrops fall across the screen and are occluded by the 3D scene geometry
/// via the Z-Buffer. If a drop's Z-depth exceeds the geometry's depth, it registers
/// a hit and draws a splash ripple on the surface.
pub fn apply_precipitation(fb: &mut Framebuffer, zb: &ZBuffer, config: &PrecipitationConfig) {
    STATE.with(|state_ref| {
        let mut state = state_ref.borrow_mut();
        let PrecipitationState { drops, rng } = &mut *state;

        let width = fb.width() as f32;
        let height = fb.height() as f32;
        let iw = fb.width() as i32;
        let ih = fb.height() as i32;

        // Spawn new drops up to max_drops
        drops.reserve_exact(config.max_drops.saturating_sub(drops.len()));
        while drops.len() < config.max_drops {
            let x = rng.next_f32() * width;
            let y = rng.next_f32() * height; // initial spawn scattered
            let z = config.z_min + rng.next_f32() * (config.z_max - config.z_min);
            let vy = config.drop_speed_min
                + rng.next_f32() * (config.drop_speed_max - config.drop_speed_min);
            let vx = config.wind * (0.8 + 0.4 * rng.next_f32());

            drops.push(Drop {
                x,
                y,
                z,
                velocity_x: vx,
                velocity_y: vy,
                state: DropState::Falling,
                life: 0.0,
                color: config.drop_color,
            });
        }

        // We use raw slices to avoid overhead of get_pixel/set_pixel in hot loops
        let pixels = fb.as_mut_slice();

        // closure to safely draw a pixel with alpha blending
        let mut draw_pixel = |x: i32, y: i32, color: u32| {
            if x >= 0 && x < iw && y >= 0 && y < ih {
                let idx = (y as usize) * (iw as usize) + (x as usize);
                let dst = pixels[idx];
                pixels[idx] = alpha_blend_pixel(color, dst);
            }
        };

        for drop in drops {
            if drop.state == DropState::Falling {
                let prev_x = drop.x as i32;
                let prev_y = drop.y as i32;

                drop.x += drop.velocity_x;
                drop.y += drop.velocity_y;
                drop.velocity_y += config.gravity;

                let ix = drop.x as i32;
                let iy = drop.y as i32;

                if iy >= ih || ix < 0 || ix >= iw {
                    // Out of bounds, respawn at the top
                    drop.y = -10.0;
                    drop.x = rng.next_f32() * width;
                    drop.z = config.z_min + rng.next_f32() * (config.z_max - config.z_min);
                    drop.velocity_y = config.drop_speed_min
                        + rng.next_f32() * (config.drop_speed_max - config.drop_speed_min);
                    drop.velocity_x = config.wind * (0.8 + 0.4 * rng.next_f32());
                } else if ix >= 0 && iy >= 0 {
                    // Check Z-buffer collision
                    if let Some(scene_z) = zb.get_depth(ix, iy) {
                        if drop.z > scene_z {
                            // Hit! Transition to splashing
                            drop.state = DropState::Splashing;
                            drop.life = config.splash_duration;
                        } else {
                            // Draw falling drop streak
                            draw_pixel(ix, iy, drop.color);
                            // Tail
                            draw_pixel(prev_x, prev_y, scale_alpha(drop.color, 0.5));
                        }
                    } else {
                        // ZBuffer is infinite or missing here
                        draw_pixel(ix, iy, drop.color);
                        draw_pixel(prev_x, prev_y, scale_alpha(drop.color, 0.5));
                    }
                }
            } else if drop.state == DropState::Splashing {
                drop.life -= 1.0;

                let ix = drop.x as i32;
                let iy = drop.y as i32;

                if drop.life <= 0.0 {
                    // Respawn
                    drop.y = -10.0;
                    drop.x = rng.next_f32() * width;
                    drop.z = config.z_min + rng.next_f32() * (config.z_max - config.z_min);
                    drop.velocity_y = config.drop_speed_min
                        + rng.next_f32() * (config.drop_speed_max - config.drop_speed_min);
                    drop.velocity_x = config.wind * (0.8 + 0.4 * rng.next_f32());
                    drop.state = DropState::Falling;
                } else {
                    // Draw ripple/splash (horizontal spread)
                    let radius = (config.splash_duration - drop.life).max(1.0) as i32;
                    let alpha_factor = drop.life / config.splash_duration;
                    let splash_col = scale_alpha(config.splash_color, alpha_factor);

                    draw_pixel(ix - radius, iy, splash_col);
                    draw_pixel(ix + radius, iy, splash_col);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raindrop_collision() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let mut zb = ZBuffer::new(100, 100).unwrap();

        // Draw a dummy surface in the Z-buffer at depth 10.0
        // Surface is spanning the whole width at y = 50..100
        for y in 50..100 {
            for x in 0..100 {
                // Bounds are statically known to be safe
                zb.as_mut_slice()[y * 100 + x] = 10.0;
            }
        }

        let config = PrecipitationConfig {
            max_drops: 1,
            z_min: 15.0, // Drop is behind the surface
            z_max: 15.0,
            gravity: 0.0,
            ..Default::default()
        };

        // Reset state
        STATE.with(|state| {
            state.borrow_mut().drops.clear();
        });

        // Run one frame to spawn the drop
        apply_precipitation(&mut fb, &zb, &config);

        STATE.with(|state| {
            let mut st = state.borrow_mut();
            assert_eq!(st.drops.len(), 1);
            let drop = &mut st.drops[0];
            // Manually position drop right above the surface
            drop.x = 50.0;
            drop.y = 49.0;
            drop.velocity_y = 2.0; // will cross y=50 next frame
            drop.velocity_x = 0.0;
            drop.state = DropState::Falling;
        });

        // Frame 2: drop falls into y=51, where z is 10.0. drop.z is 15.0.
        // 15.0 > 10.0 -> SPLASH
        apply_precipitation(&mut fb, &zb, &config);

        STATE.with(|state| {
            let st = state.borrow();
            assert_eq!(st.drops[0].state, DropState::Splashing);
        });
    }
}
