use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash_render::experimental::attractors::{
    AizawaAttractor, LorenzAttractor, RoesslerAttractor, render_attractor,
};

struct AttractorsDemo {
    fb: Framebuffer,
    lorenz: LorenzAttractor,
    roessler: RoesslerAttractor,
    aizawa: AizawaAttractor,
    pitch: f32,
    yaw: f32,
    current_attractor: usize,
    auto_rotate: bool,
    presenter: Option<SoftwarePresenter>,
}

impl WindowApp for AttractorsDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            width: 800,
            height: 600,
            title: "Strange Attractors Demo".to_string(),
            ..Default::default()
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        self.fb.clear(0xFF_00_00_00);
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        if self.auto_rotate {
            self.yaw += 0.02;
            self.pitch += 0.01;
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Draw background but don't fully clear to leave trails
        // Add a slight fade effect by drawing a semi-transparent black rect over the screen
        let pixels = self.fb.as_mut_slice();
        for pixel in pixels.iter_mut() {
            let p = *pixel;
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;

            // Fade slightly
            let r = r.saturating_sub(5);
            let g = g.saturating_sub(5);
            let b = b.saturating_sub(5);

            *pixel = 0xFF_00_00_00 | (r << 16) | (g << 8) | b;
        }

        match self.current_attractor {
            0 => {
                // Lorenz
                render_attractor(
                    &mut self.fb,
                    &mut self.lorenz,
                    1500,
                    0.005,
                    15.0,
                    self.pitch,
                    self.yaw,
                );
            }
            1 => {
                // Rössler
                render_attractor(
                    &mut self.fb,
                    &mut self.roessler,
                    1500,
                    0.01,
                    20.0,
                    self.pitch,
                    self.yaw,
                );
            }
            2 => {
                // Aizawa
                render_attractor(
                    &mut self.fb,
                    &mut self.aizawa,
                    1500,
                    0.01,
                    100.0,
                    self.pitch,
                    self.yaw,
                );
            }
            _ => {}
        }

        if let Some(ref mut presenter) = self.presenter {
            presenter.present(&self.fb)?;
        }
        Ok(())
    }

    fn input(
        &mut self,
        _ctx: WindowContext<'_>,
        event: &winit::event::WindowEvent,
    ) -> Result<(), Self::Error> {
        if let winit::event::WindowEvent::KeyboardInput { event, .. } = event {
            if event.state != winit::event::ElementState::Pressed {
                return Ok(());
            }
            match event.physical_key {
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Space) => {
                    self.auto_rotate = !self.auto_rotate;
                }
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Digit1) => {
                    self.current_attractor = 0;
                    self.fb.clear(0xFF_00_00_00);
                }
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Digit2) => {
                    self.current_attractor = 1;
                    self.fb.clear(0xFF_00_00_00);
                }
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Digit3) => {
                    self.current_attractor = 2;
                    self.fb.clear(0xFF_00_00_00);
                }
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ArrowLeft) => {
                    self.yaw -= 0.1;
                }
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ArrowRight) => {
                    self.yaw += 0.1;
                }
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ArrowUp) => {
                    self.pitch += 0.1;
                }
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ArrowDown) => {
                    self.pitch -= 0.1;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn main() {
    let demo = AttractorsDemo {
        fb: Framebuffer::new(800, 600).unwrap(),
        lorenz: LorenzAttractor::new(Vec3::new(0.1, 0.0, 0.0)),
        roessler: RoesslerAttractor::new(Vec3::new(0.1, 0.0, 0.0)),
        aizawa: AizawaAttractor::new(Vec3::new(0.1, 0.0, 0.0)),
        pitch: 0.0,
        yaw: 0.0,
        current_attractor: 0,
        auto_rotate: true,
        presenter: None,
    };

    println!("Attractors Demo:");
    println!("  1: Lorenz Attractor");
    println!("  2: Rössler Attractor");
    println!("  3: Aizawa Attractor");
    println!("  Space: Toggle Auto-Rotate");
    println!("  Arrows: Manual Rotate");

    run_windowed(demo);
}
