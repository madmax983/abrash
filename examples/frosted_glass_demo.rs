use abrash::experimental::frosted_glass::apply_frosted_glass;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const RADIUS: u32 = 4;

struct FrostedGlassDemoApp {
    time: f32,
    base_image: Framebuffer,
    presenter: Option<SoftwarePresenter>,
}

impl FrostedGlassDemoApp {
    fn new() -> Self {
        let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
        // Draw a test image to apply frosted glass on.
        // E.g. simple colorful circles or squares
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let r = (x % 256) as u32;
                let g = (y % 256) as u32;
                let b = ((x + y) % 256) as u32;
                fb.set_pixel(x as i32, y as i32, 0xFF00_0000 | (r << 16) | (g << 8) | b);
            }
        }

        // Add some distinct shapes
        for r in 0..50 {
            for t in 0..360 {
                let rad = (t as f32).to_radians();
                let dx = (rad.cos() * r as f32) as i32;
                let dy = (rad.sin() * r as f32) as i32;

                let cx = 400;
                let cy = 300;

                if cx + dx >= 0 && cx + dx < WIDTH as i32 && cy + dy >= 0 && cy + dy < HEIGHT as i32 {
                    fb.set_pixel((cx + dx) as i32, (cy + dy) as i32, 0xFFFF_00FF);
                }
            }
        }

        Self {
            time: 0.0,
            base_image: fb,
            presenter: None,
        }
    }
}

impl WindowApp for FrostedGlassDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Frosted Glass Filter Demo".to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Just rough time approximation based on expected 60fps for visual effect
        self.time += 1.0 / 60.0;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Copy base image into current framebuffer to avoid permanent blur build-up
        let mut cloned_fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
        cloned_fb.as_mut_slice().copy_from_slice(self.base_image.as_slice());

        // Compute radius based on time or mouse pos
        let radius = ((self.time * 2.0).sin() * 5.0 + RADIUS as f32).clamp(0.0, 15.0) as u32;

        // Apply frosted glass
        let seed = (self.time * 1000.0) as u64; // animate noise
        apply_frosted_glass(&mut cloned_fb, radius, seed);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&cloned_fb)?;
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_windowed(FrostedGlassDemoApp::new())?;
    Ok(())
}
