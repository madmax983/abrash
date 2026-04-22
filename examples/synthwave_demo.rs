#[cfg(feature = "nova")]
use abrash::experimental::synthwave::{SynthwaveConfig, apply_synthwave};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{WindowApp, WindowContext, run_app};

struct SynthwaveApp {
    #[cfg(feature = "nova")]
    config: SynthwaveConfig,
    time: f32,
}

impl SynthwaveApp {
    fn new() -> Self {
        Self {
            #[cfg(feature = "nova")]
            config: SynthwaveConfig::default(),
            time: 0.0,
        }
    }

    fn draw_sun(&self, fb: &mut Framebuffer) {
        let width = fb.width() as i32;
        let height = fb.height() as i32;

        // Draw the sky
        let pixels = fb.as_mut_slice();
        for pixel in pixels.iter_mut() {
            *pixel = 0xFF100020; // Dark purple sky
        }

        let cx = width / 2;
        let cy = (height as f32 * 0.4) as i32; // Just above horizon
        let radius = (height as f32 * 0.25) as i32;

        let sun_color_top = 0xFFFFD700; // Gold
        let sun_color_bottom = 0xFFFF00FF; // Magenta

        for y in (cy - radius)..=(cy + radius) {
            for x in (cx - radius)..=(cx + radius) {
                let dx = x - cx;
                let dy = y - cy;
                if dx * dx + dy * dy <= radius * radius {
                    // Create horizontal slices for the retro sun look
                    // The slices get thicker towards the bottom
                    let norm_y = (y - (cy - radius)) as f32 / (2.0 * radius as f32);

                    // Slices logic: draw unless we are in a gap
                    let slice_frequency = 10.0;
                    let val = (norm_y * slice_frequency - self.time * 2.0).fract();

                    let gap_thickness = norm_y * 0.6; // Gaps get bigger at the bottom

                    if val > gap_thickness || norm_y < 0.3 {
                        if x >= 0 && x < width && y >= 0 && y < height {
                            // Interpolate color from top to bottom
                            let r1 = ((sun_color_top >> 16) & 0xFF) as f32;
                            let g1 = ((sun_color_top >> 8) & 0xFF) as f32;
                            let b1 = (sun_color_top & 0xFF) as f32;

                            let r2 = ((sun_color_bottom >> 16) & 0xFF) as f32;
                            let g2 = ((sun_color_bottom >> 8) & 0xFF) as f32;
                            let b2 = (sun_color_bottom & 0xFF) as f32;

                            let r = (r1 + (r2 - r1) * norm_y) as u32;
                            let g = (g1 + (g2 - g1) * norm_y) as u32;
                            let b = (b1 + (b2 - b1) * norm_y) as u32;

                            let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

                            unsafe {
                                fb.set_pixel_unchecked(x as usize, y as usize, color);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl WindowApp for SynthwaveApp {
    fn update(&mut self, ctx: WindowContext<'_>) {
        self.time += ctx.dt_seconds;
        #[cfg(feature = "nova")]
        {
            self.config.time = self.time;
        }
    }

    fn render(&mut self, fb: &mut Framebuffer) {
        self.draw_sun(fb);
        #[cfg(feature = "nova")]
        apply_synthwave(fb, &self.config);
    }
}

fn main() {
    let app = SynthwaveApp::new();
    run_app(app, 800, 600, "Retro Synthwave Generator");
}
