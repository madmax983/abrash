use abrash::experimental::falling_sand::{FallingSand, ParticleType};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use rand::Rng;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

struct FallingSandDemo {
    sim: FallingSand,
    brush_size: usize,
    current_element: ParticleType,
    mouse_down: bool,
    mouse_x: f32,
    mouse_y: f32,
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
}

impl FallingSandDemo {
    fn new(width: usize, height: usize) -> Self {
        let mut sim = FallingSand::new(width, height);

        // Add some ground
        for x in 0..width {
            for y in height - 20..height {
                sim.set_particle(x, y, ParticleType::Wall, 0xFF444444);
            }
        }

        // Add some obstacles
        for x in 300..500 {
            sim.set_particle(x, 400, ParticleType::Wall, 0xFF888888);
        }

        Self {
            sim,
            brush_size: 10,
            current_element: ParticleType::Sand,
            mouse_down: false,
            mouse_x: 0.0,
            mouse_y: 0.0,
            presenter: None,
            framebuffer: Framebuffer::new(width as u32, height as u32).unwrap(),
        }
    }

    fn spawn_particles(&mut self, x: usize, y: usize) {
        let mut rng = rand::thread_rng();
        let half_brush = self.brush_size / 2;

        for dy in 0..self.brush_size {
            for dx in 0..self.brush_size {
                let px = (x as i32 + dx as i32 - half_brush as i32).max(0) as usize;
                let py = (y as i32 + dy as i32 - half_brush as i32).max(0) as usize;

                if px < self.sim.width && py < self.sim.height {
                    // Make it a bit random so it's not a solid square
                    if rng.gen_bool(0.3) {
                        let color = match self.current_element {
                            ParticleType::Sand => {
                                let shade = rng.gen_range(200..255);
                                0xFF000000 | (shade << 16) | (shade << 8) | (shade / 2)
                            }
                            ParticleType::Water => {
                                let shade = rng.gen_range(150..255);
                                0xFF000000 | ((shade / 2) << 8) | shade
                            }
                            ParticleType::Wall => 0xFF888888,
                            ParticleType::Empty => 0x00000000,
                        };
                        self.sim.set_particle(px, py, self.current_element, color);
                    }
                }
            }
        }
    }

    fn present(&mut self) -> Result<(), HostError> {
        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }
        Ok(())
    }
}

impl WindowApp for FallingSandDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Abrash - Falling Sand".to_string(),
            width: 800,
            height: 600,
            vsync: true,
            ..Default::default()
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn input(&mut self, _ctx: WindowContext<'_>, event: &WindowEvent) -> Result<(), Self::Error> {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && !event.repeat {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Digit1) => {
                            self.current_element = ParticleType::Sand;
                        }
                        PhysicalKey::Code(KeyCode::Digit2) => {
                            self.current_element = ParticleType::Water;
                        }
                        PhysicalKey::Code(KeyCode::Digit3) => {
                            self.current_element = ParticleType::Wall;
                        }
                        PhysicalKey::Code(KeyCode::Digit4) => {
                            self.current_element = ParticleType::Empty;
                        }
                        PhysicalKey::Code(KeyCode::KeyR) => {
                            self.sim = FallingSand::new(self.sim.width, self.sim.height);
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as f32;
                self.mouse_y = position.y as f32;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    self.mouse_down = *state == ElementState::Pressed;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        if self.mouse_down {
            self.spawn_particles(self.mouse_x as usize, self.mouse_y as usize);
        }
        self.sim.update();
        ctx.window.request_redraw();
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(0xFF111111);
        self.sim.draw(&mut self.framebuffer);

        self.present()
    }
}

fn main() -> Result<(), HostError> {
    let width = 800;
    let height = 600;
    let app = FallingSandDemo::new(width, height);
    run_windowed(app)
}
