use abrash::experimental::fractal::{MandelbrotConfig, render_mandelbrot};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use std::error::Error;
use winit::event::WindowEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

struct FractalApp {
    framebuffer: Framebuffer,
    presenter: Option<SoftwarePresenter>,
    config: MandelbrotConfig,
}

impl FractalApp {
    fn new() -> Self {
        Self {
            framebuffer: Framebuffer::new(800, 600).unwrap(),
            presenter: None,
            config: MandelbrotConfig::default(),
        }
    }
}

// Custom error type that implements Send + Sync
#[derive(Debug)]
struct FractalError(String);

impl std::fmt::Display for FractalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for FractalError {}

impl WindowApp for FractalApp {
    type Error = FractalError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "🌟 Nova: Mandelbrot Explorer".to_string(),
            width: 800,
            height: 600,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(
            SoftwarePresenter::new(ctx.window.clone()).map_err(|e| FractalError(e.to_string()))?,
        );
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Render the fractal
        render_mandelbrot(&mut self.framebuffer, &self.config);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Present the rendered buffer to the window
        if let Some(presenter) = &mut self.presenter {
            let _ = presenter.present(&self.framebuffer);
        }
        Ok(())
    }

    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        if width > 0 && height > 0 {
            if let Ok(fb) = Framebuffer::new(width, height) {
                self.framebuffer = fb;
            }
        }
        Ok(())
    }

    fn input(&mut self, _ctx: WindowContext<'_>, event: &WindowEvent) -> Result<(), Self::Error> {
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if event.state.is_pressed() {
                match event.physical_key {
                    // Zoom
                    PhysicalKey::Code(KeyCode::KeyQ)
                    | PhysicalKey::Code(KeyCode::Equal)
                    | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                        self.config.zoom *= 1.1;
                    }
                    PhysicalKey::Code(KeyCode::KeyE)
                    | PhysicalKey::Code(KeyCode::Minus)
                    | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                        self.config.zoom /= 1.1;
                    }
                    // Pan
                    PhysicalKey::Code(KeyCode::ArrowLeft) | PhysicalKey::Code(KeyCode::KeyA) => {
                        self.config.center_x -= 0.1 / self.config.zoom;
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight) | PhysicalKey::Code(KeyCode::KeyD) => {
                        self.config.center_x += 0.1 / self.config.zoom;
                    }
                    PhysicalKey::Code(KeyCode::ArrowUp) | PhysicalKey::Code(KeyCode::KeyW) => {
                        self.config.center_y -= 0.1 / self.config.zoom;
                    }
                    PhysicalKey::Code(KeyCode::ArrowDown) | PhysicalKey::Code(KeyCode::KeyS) => {
                        self.config.center_y += 0.1 / self.config.zoom;
                    }
                    // Iteration depth
                    PhysicalKey::Code(KeyCode::KeyI) => {
                        self.config.max_iterations = self.config.max_iterations.saturating_add(10);
                    }
                    PhysicalKey::Code(KeyCode::KeyK) => {
                        self.config.max_iterations =
                            self.config.max_iterations.saturating_sub(10).max(10);
                    }
                    // Color rotation
                    PhysicalKey::Code(KeyCode::KeyC) => {
                        self.config.color_mult += 0.01;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

fn main() {
    println!("Mandelbrot Explorer Controls:");
    println!("  Pan:  WASD / Arrows");
    println!("  Zoom: Q/E or +/-");
    println!("  Iter: I/K to increase/decrease detail");
    println!("  Color: C to cycle colors");

    let app = FractalApp::new();
    run_windowed(app);
}
