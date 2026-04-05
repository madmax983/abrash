use abrash::experimental::falling_sand::{Material, SandGrid};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use winit::event::{ElementState, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};

#[cfg(feature = "nova")]
use comfy_table::{Cell, Color, Table, presets};
#[cfg(feature = "nova")]
use crossterm::style::Stylize;

#[cfg(feature = "nova")]
fn print_banner() {
    println!("\n{}", "🌟 Falling Sand Demo".bold().cyan());
    println!("{}", "====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Cellular Automata Particle Physics").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![Cell::new("Left Click"), Cell::new("Spawn Material")])
        .add_row(vec![Cell::new("Right Click"), Cell::new("Erase (Spawn Empty)")])
        .add_row(vec![Cell::new("1"), Cell::new("Select Sand").fg(Color::Yellow)])
        .add_row(vec![Cell::new("2"), Cell::new("Select Water").fg(Color::Blue)])
        .add_row(vec![Cell::new("3"), Cell::new("Select Wall").fg(Color::DarkGrey)])
        .add_row(vec![Cell::new("C"), Cell::new("Clear Board")]);
    println!("{controls}\n");
}

const WIDTH: u32 = 320; // Lower resolution for better performance & chunkier pixels
const HEIGHT: u32 = 240;
const TITLE: &str = "Nova: Falling Sand Demo";

struct FallingSandApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    grid: SandGrid,
    selected_material: Material,
    mouse_x: f32,
    mouse_y: f32,
    is_left_clicking: bool,
    is_right_clicking: bool,
    brush_size: i32,
}

impl FallingSandApp {
    fn new() -> Result<Self, HostError> {
        let grid = SandGrid::new(WIDTH, HEIGHT);

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            grid,
            selected_material: Material::Sand,
            mouse_x: 0.0,
            mouse_y: 0.0,
            is_left_clicking: false,
            is_right_clicking: false,
            brush_size: 5,
        })
    }

    fn spawn_circle(&mut self, center_x: i32, center_y: i32, mat: Material) {
        let radius = self.brush_size;
        for y in -radius..=radius {
            for x in -radius..=radius {
                if x * x + y * y <= radius * radius {
                    // Add some noise if it's sand/water to make it look less rigid
                    if mat == Material::Wall || mat == Material::Empty || (x.wrapping_mul(13) + y.wrapping_mul(7)) % 3 != 0 {
                        self.grid.set_material(center_x + x, center_y + y, mat);
                    }
                }
            }
        }
    }
}

impl WindowApp for FallingSandApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: WIDTH * 2, // Scale up window for visibility
            height: HEIGHT * 2,
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

    fn input(&mut self, _ctx: WindowContext<'_>, event: &winit::event::WindowEvent) -> Result<(), Self::Error> {
        match event {
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as f32 / 2.0;
                self.mouse_y = position.y as f32 / 2.0;
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    self.is_left_clicking = *state == ElementState::Pressed;
                } else if *button == MouseButton::Right {
                    self.is_right_clicking = *state == ElementState::Pressed;
                }
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Digit1) => self.selected_material = Material::Sand,
                        PhysicalKey::Code(KeyCode::Digit2) => self.selected_material = Material::Water,
                        PhysicalKey::Code(KeyCode::Digit3) => self.selected_material = Material::Wall,
                        PhysicalKey::Code(KeyCode::KeyC) => {
                            self.grid = SandGrid::new(WIDTH, HEIGHT);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Handle continuous spawning
        if self.is_left_clicking {
            self.spawn_circle(self.mouse_x as i32, self.mouse_y as i32, self.selected_material);
        } else if self.is_right_clicking {
            self.spawn_circle(self.mouse_x as i32, self.mouse_y as i32, Material::Empty);
        }

        // Step simulation multiple times per frame for faster fluid flow
        self.grid.step();
        self.grid.step();

        // Draw the simulation to the framebuffer
        self.grid.draw(&mut self.framebuffer);

        // UI crosshair
        let mx = self.mouse_x as i32;
        let my = self.mouse_y as i32;
        if mx >= 0 && mx < WIDTH as i32 && my >= 0 && my < HEIGHT as i32 {
            let color = match self.selected_material {
                Material::Sand => 0xFF_FF_FF_00,
                Material::Water => 0xFF_00_FF_FF,
                Material::Wall => 0xFF_FF_00_00,
                _ => 0xFF_FF_FF_FF,
            };
            let r = self.brush_size;
            // Draw a rough circle outline for brush
            for i in -r..=r {
                self.framebuffer.set_pixel(mx + i, my - r, color);
                self.framebuffer.set_pixel(mx + i, my + r, color);
                self.framebuffer.set_pixel(mx - r, my + i, color);
                self.framebuffer.set_pixel(mx + r, my + i, color);
            }
        }

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
    #[cfg(feature = "nova")]
    print_banner();

    run_windowed(FallingSandApp::new().unwrap())
}
