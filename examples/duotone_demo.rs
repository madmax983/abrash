use abrash::platform::{run_windowed, WindowApp, WindowContext, WindowHostConfig};
use abrash_render::experimental::duotone::{apply_duotone, DuotoneConfig};
use std::convert::Infallible;

struct DuotoneDemo {
    time: f32,
    config: DuotoneConfig,
}

impl DuotoneDemo {
    fn new() -> Self {
        Self {
            time: 0.0,
            config: DuotoneConfig::default(), // Blue / Orange
        }
    }
}

impl WindowApp for DuotoneDemo {
    type Error = Infallible;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Duotone Demo".to_string(),
            width: 800,
            height: 600,
            ..Default::default()
        }
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds;

        // Slowly oscillate between colors
        let r = ((self.time.sin() + 1.0) * 127.5) as u32;
        let b = ((self.time.cos() + 1.0) * 127.5) as u32;

        self.config.color_dark = 0xFF000000 | (r << 16) | b;
        Ok(())
    }

    fn render(&mut self, fb: &mut abrash::framebuffer::Framebuffer) -> Result<(), Self::Error> {
        fb.clear(0xFF222222);

        let w = fb.width() as i32;
        let h = fb.height() as i32;

        // Draw some shapes with various grays and colors
        for y in 0..h {
            for x in 0..w {
                // Background pattern
                let chess = ((x / 32) + (y / 32)) % 2;
                if chess == 0 {
                    fb.set_pixel(x, y, 0xFF444444);
                }

                // A colorful gradient circle in the middle
                let cx = w / 2;
                let cy = h / 2;
                let dx = x - cx;
                let dy = y - cy;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq < 100 * 100 {
                    let r = ((x as f32 / w as f32) * 255.0) as u32;
                    let g = ((y as f32 / h as f32) * 255.0) as u32;
                    fb.set_pixel(x, y, 0xFF000000 | (r << 16) | (g << 8) | 0x80);
                }
            }
        }

        // Apply the duotone effect on top of everything
        apply_duotone(fb, &self.config);
        Ok(())
    }
}

fn main() {
    run_windowed(DuotoneDemo::new());
}
