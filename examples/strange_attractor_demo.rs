//! Strange Attractor Demo
use abrash::experimental::strange_attractor::{AttractorType, Particle, StrangeAttractor};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_core::math::Vec3;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "🌟 Nova: Strange Attractor Demo";

struct AttractorDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    sim: StrangeAttractor,
}

impl AttractorDemo {
    fn new() -> Result<Self, HostError> {
        let mut particles = Vec::with_capacity(5000);
        for i in 0..5000 {
            particles.push(Particle {
                position: Vec3::new(0.1 + (i as f32 * 0.001), 0.1, 0.1),
                color: 0xFF_00FF00,
            });
        }
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            sim: StrangeAttractor::new(
                particles,
                AttractorType::Lorenz {
                    sigma: 10.0,
                    rho: 28.0,
                    beta: 8.0 / 3.0,
                },
                0.005,
            ),
        })
    }

    fn present(&mut self) -> Result<(), HostError> {
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::App("software presenter not initialized".to_string()))?;
        presenter.present(&self.framebuffer)?;
        Ok(())
    }
}

impl WindowApp for AttractorDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
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
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF_00_00_00);
        self.sim.run_steps(5);

        for p in &self.sim.particles {
            let sx = (p.position.x * 8.0) as i32 + (WIDTH / 2) as i32;
            let sy = (p.position.z * 8.0) as i32 + (HEIGHT / 2) as i32;

            if sx >= 0 && sx < WIDTH as i32 && sy >= 0 && sy < HEIGHT as i32 {
                self.framebuffer.set_pixel(sx, sy, p.color);
            }
        }

        self.present()
    }
}

fn main() {
    run_windowed(AttractorDemo::new().unwrap());
}
