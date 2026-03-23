use abrash::experimental::tilt_shift::{TiltShiftConfig, apply_tilt_shift};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

fn generate_procedural_city(fb: &mut Framebuffer) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;

    // Draw sky
    for y in 0..height {
        let t = y as f32 / height as f32;
        let r = ((1.0 - t) * 100.0 + t * 200.0) as u32;
        let g = ((1.0 - t) * 150.0 + t * 220.0) as u32;
        let b = ((1.0 - t) * 255.0 + t * 255.0) as u32;
        let sky_col = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        for x in 0..width {
            fb.set_pixel(x, y, sky_col);
        }
    }

    // Draw some fake buildings (rectangles)
    let buildings = [
        (100, 300, 50, 200, 0xFF40_4040),
        (200, 250, 80, 250, 0xFF60_6060),
        (350, 150, 60, 350, 0xFF30_3030),
        (450, 350, 100, 150, 0xFF50_5050),
        (600, 200, 70, 300, 0xFF70_7070),
    ];

    for &(bx, by, bw, bh, col) in &buildings {
        fb.clear_rect(bx, by, bw as u32, bh as u32, col);

        // Add fake windows
        for wy in (by + 10..by + bh - 10).step_by(20) {
            for wx in (bx + 10..bx + bw - 10).step_by(15) {
                fb.clear_rect(wx, wy, 8, 12, 0xFFFF_FF00); // Yellow lit windows
            }
        }
    }

    // Draw foreground ground/street
    fb.clear_rect(0, 400, width as u32, (height - 400) as u32, 0xFF20_2020);

    // Draw street lines
    for x in (0..width).step_by(60) {
        fb.clear_rect(x, 480, 30, 10, 0xFFDD_DDDD);
    }
}

struct TiltShiftApp {
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    config: TiltShiftConfig,
}

impl TiltShiftApp {
    fn new() -> Result<Self, HostError> {
        let width = 800;
        let height = 600;

        let framebuffer = Framebuffer::new(width, height)
            .map_err(|e| HostError::App(format!("Failed to create framebuffer: {:?}", e)))?;

        Ok(Self {
            framebuffer,
            presenter: None,
            config: TiltShiftConfig {
                focus_dist: 0.5,
                focus_range: 0.1,
                blur_radius: 8,
            },
        })
    }
}

impl WindowApp for TiltShiftApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Tilt Shift Demo".to_string(),
            width: 800,
            height: 600,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // For this simple demo, we just animate the focus point slowly
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32();
        self.config.focus_dist = 0.5 + (time * 0.5).sin() * 0.3;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Render base image
        generate_procedural_city(&mut self.framebuffer);

        // Apply effect
        apply_tilt_shift(&mut self.framebuffer, &self.config);

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

fn main() -> Result<(), HostError> {
    println!("Controls:");
    println!("  Up/Down: Move Focus Plane");
    println!("  Left/Right: Change Blur Radius");

    run_windowed(TiltShiftApp::new()?)
}
