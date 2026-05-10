use abrash_core::math::Vec3;
use abrash_render::experimental::strange_attractor::{
    render_strange_attractor, AttractorConfig, AttractorType,
};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Strange Attractor Demo";

struct StrangeAttractorDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    time: f32,
    attractor_type: AttractorType,
}

impl StrangeAttractorDemo {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            time: 0.0,
            attractor_type: AttractorType::Lorenz {
                sigma: 10.0,
                rho: 28.0,
                beta: 8.0 / 3.0,
            },
        })
    }
}

impl WindowApp for StrangeAttractorDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: self.framebuffer.width(),
            height: self.framebuffer.height(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds;

        // Cycle attractors every 5 seconds
        let cycle = (self.time / 5.0) as usize % 3;
        self.attractor_type = match cycle {
            0 => AttractorType::Lorenz {
                sigma: 10.0,
                rho: 28.0,
                beta: 8.0 / 3.0,
            },
            1 => AttractorType::Thomas { b_param: 0.208186 },
            _ => AttractorType::Aizawa {
                a: 0.95,
                b: 0.7,
                c: 0.6,
                d: 3.5,
                e: 0.25,
                f: 0.1,
            },
        };
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // We do a trailing fade effect
        let pixels = self.framebuffer.as_mut_slice();
        for pixel in pixels.iter_mut() {
            let r = ((*pixel >> 16) & 0xFF).saturating_sub(5);
            let g = ((*pixel >> 8) & 0xFF).saturating_sub(5);
            let b = (*pixel & 0xFF).saturating_sub(5);
            *pixel = 0xFF_000000 | (r << 16) | (g << 8) | b;
        }

        let config = match self.attractor_type {
            AttractorType::Lorenz { .. } => AttractorConfig {
                attractor_type: self.attractor_type,
                iterations: 50_000,
                dt: 0.005,
                scale: 12.0,
                start_pos: Vec3::new(0.1, 0.0, 0.0),
                color: 0xFF_0055FF, // Blueish
                additive: true,
            },
            AttractorType::Thomas { .. } => AttractorConfig {
                attractor_type: self.attractor_type,
                iterations: 100_000,
                dt: 0.05,
                scale: 50.0,
                start_pos: Vec3::new(0.1, 0.0, 0.0),
                color: 0xFF_FF5500, // Orange
                additive: true,
            },
            AttractorType::Aizawa { .. } => AttractorConfig {
                attractor_type: self.attractor_type,
                iterations: 100_000,
                dt: 0.01,
                scale: 100.0,
                start_pos: Vec3::new(0.1, 0.0, 0.0),
                color: 0xFF_00FF55, // Green
                additive: true,
            },
        };

        render_strange_attractor(&mut self.framebuffer, &config);

        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

fn main() {
    run_windowed(StrangeAttractorDemo::new().unwrap());
}
