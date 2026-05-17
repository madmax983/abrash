use abrash::experimental::tiny_planet::{TinyPlanetConfig, apply_tiny_planet};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

struct TinyPlanetDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    config: TinyPlanetConfig,
    time: f32,
    original_scene: Framebuffer,
}

#[derive(Debug)]
struct DemoError(String);

impl std::fmt::Display for DemoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for DemoError {}

impl From<abrash::platform::HostError> for DemoError {
    fn from(err: abrash::platform::HostError) -> Self {
        Self(err.to_string())
    }
}

impl WindowApp for TinyPlanetDemo {
    type Error = DemoError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Tiny Planet Filter - Abrash".to_string(),
            width: self.framebuffer.width(),
            height: self.framebuffer.height(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);

        let width = self.framebuffer.width() as usize;
        let height = self.framebuffer.height() as usize;
        let pixels = self.original_scene.as_mut_slice();

        for y in 0..height {
            for x in 0..width {
                let horizon = height / 2;
                if y < horizon {
                    let t = y as f32 / horizon as f32;
                    let r = (100.0 * (1.0 - t) + 20.0 * t) as u32;
                    let g = (150.0 * (1.0 - t) + 50.0 * t) as u32;
                    let b = (255.0 * (1.0 - t) + 150.0 * t) as u32;
                    pixels[y * width + x] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                } else {
                    pixels[y * width + x] = 0xFF_228B22;
                }
            }
        }

        let sun_x = width / 4;
        let sun_y = height / 4;
        let sun_radius = 40.0;
        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - sun_x as f32;
                let dy = y as f32 - sun_y as f32;
                if dx.hypot(dy) < sun_radius {
                    pixels[y * width + x] = 0xFF_FFD700;
                }
            }
        }

        for i in 0..10 {
            let bx = (width / 10) * i + (i * 15 % 50);
            let bw = 20 + (i * 7 % 30);
            let bh = 40 + (i * 11 % 80);
            let by = height / 2 - bh;

            for y in by..height / 2 {
                for x in bx..bx + bw {
                    if x < width {
                        pixels[y * width + x] = 0xFF_8B4513;
                    }
                }
            }
        }

        Ok(())
    }

    fn input(&mut self, _ctx: WindowContext<'_>, event: &WindowEvent) -> Result<(), Self::Error> {
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(keycode),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = event
        {
            match keycode {
                KeyCode::ArrowUp => self.config.zoom -= 0.1,
                KeyCode::ArrowDown => self.config.zoom += 0.1,
                KeyCode::ArrowLeft => self.config.horizon_offset -= 0.05,
                KeyCode::ArrowRight => self.config.horizon_offset += 0.05,
                _ => {}
            }
            self.config.zoom = self.config.zoom.max(0.1);
        }
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += 0.016; // approximation
        self.config.rotation = self.time * 0.5;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer
            .as_mut_slice()
            .copy_from_slice(self.original_scene.as_slice());
        apply_tiny_planet(&mut self.framebuffer, &self.config);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }

        Ok(())
    }
}

fn main() -> Result<(), String> {
    let width = 800;
    let height = 600;

    let app = TinyPlanetDemo {
        presenter: None,
        framebuffer: Framebuffer::new(width, height)?,
        config: TinyPlanetConfig::default(),
        time: 0.0,
        original_scene: Framebuffer::new(width, height)?,
    };

    run_windowed(app);
    Ok(())
}
