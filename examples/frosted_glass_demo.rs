use abrash::framebuffer::Framebuffer;
use abrash::platform::{run_windowed, WindowApp, WindowContext, WindowHostConfig};
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use abrash_render::experimental::frosted_glass::apply_frosted_glass;

#[derive(Debug)]
struct StringError(String);

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for StringError {}

struct FrostedGlassDemo {
    fb: Framebuffer,
    radius: u32,
    config: WindowHostConfig,
}

impl FrostedGlassDemo {
    fn new(width: u32, height: u32) -> Self {
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Draw a simple pattern to show the effect
        for y in 0..height {
            for x in 0..width {
                let cell_x = x / 50;
                let cell_y = y / 50;
                let color = if (cell_x + cell_y) % 2 == 0 {
                    0xFFFF0000 // Red
                } else {
                    0xFF0000FF // Blue
                };
                fb.set_pixel(x as i32, y as i32, color);
            }
        }

        // Draw some shapes
        for r in 10..50 {
            for a in 0..360 {
                let rad = a as f32 * std::f32::consts::PI / 180.0;
                let cx = (width / 2) as f32;
                let cy = (height / 2) as f32;
                let x = cx + (r as f32) * rad.cos();
                let y = cy + (r as f32) * rad.sin();
                fb.set_pixel(x as i32, y as i32, 0xFF00FF00); // Green circle
            }
        }

        let mut config = WindowHostConfig::default();
        config.width = width;
        config.height = height;
        config.title = "Frosted Glass Demo".to_string();

        Self {
            fb,
            radius: 10,
            config,
        }
    }
}

impl WindowApp for FrostedGlassDemo {
    type Error = StringError;

    fn config(&self) -> WindowHostConfig {
        self.config.clone()
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn render(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Redraw base image because frosted glass works in-place
        let width = self.fb.width() as u32;
        let height = self.fb.height() as u32;

        for y in 0..height {
            for x in 0..width {
                let cell_x = x / 50;
                let cell_y = y / 50;
                let color = if (cell_x + cell_y) % 2 == 0 {
                    0xFFFF0000
                } else {
                    0xFF0000FF
                };
                self.fb.set_pixel(x as i32, y as i32, color);
            }
        }
        for r in 10..50 {
            for a in 0..360 {
                let rad = a as f32 * std::f32::consts::PI / 180.0;
                let cx = (width / 2) as f32;
                let cy = (height / 2) as f32;
                let x = cx + (r as f32) * rad.cos();
                let y = cy + (r as f32) * rad.sin();
                self.fb.set_pixel(x as i32, y as i32, 0xFF00FF00);
            }
        }

        // Apply effect
        apply_frosted_glass(&mut self.fb, self.radius);

        let context = Context::new(ctx.window.clone()).map_err(|e: softbuffer::SoftBufferError| StringError(e.to_string()))?;
        let mut surface = Surface::new(&context, ctx.window.clone()).map_err(|e: softbuffer::SoftBufferError| StringError(e.to_string()))?;

        let width = NonZeroU32::new(self.fb.width() as u32).unwrap();
        let height = NonZeroU32::new(self.fb.height() as u32).unwrap();

        surface.resize(width, height).map_err(|e: softbuffer::SoftBufferError| StringError(e.to_string()))?;
        let mut buffer = surface.buffer_mut().map_err(|e: softbuffer::SoftBufferError| StringError(e.to_string()))?;
        buffer.copy_from_slice(self.fb.as_slice());
        buffer.present().map_err(|e: softbuffer::SoftBufferError| StringError(e.to_string()))?;

        Ok(())
    }
}

fn main() {
    let width = 800;
    let height = 600;
    let app = FrostedGlassDemo::new(width, height);
    run_windowed(app);
}
