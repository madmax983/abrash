use abrash::bam::{ANG90, Bam};
use abrash::fixed16_16::Fixed16_16;
use abrash::framebuffer::Framebuffer;
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed_app,
};
use abrash::raycast::map::ArrayGridMap;
use abrash::raycast::types::{Cell, Vec2Fixed};
use abrash::raycaster::hybrid::render_raycast_view;
use abrash::zbuffer::ZBuffer;

use comfy_table::{Cell as TableCell, Color, Table, presets};
use crossterm::style::Stylize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{Key, NamedKey};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF1A_1A2E; // dark blue-grey ceiling/floor
const TITLE: &str = "Abrash - Raycaster Demo";

/// Movement speed per frame in fixed-point world units.
const MOVE_SPEED: f32 = 0.05;
/// Rotation speed per frame (~11 degrees in BAM).
const TURN_SPEED: u32 = 0x0200_0000;

fn print_banner() {
    println!("\n{}", "🔦 Raycaster Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            TableCell::new("Property").fg(Color::Cyan),
            TableCell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            TableCell::new("Description"),
            TableCell::new("Wolfenstein-style raycaster").fg(Color::Green),
        ])
        .add_row(vec![
            TableCell::new("Renderer"),
            TableCell::new("DDA Raycaster + Software Presenter").fg(Color::Yellow),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            TableCell::new("Input").fg(Color::Cyan),
            TableCell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![
            TableCell::new("W / S"),
            TableCell::new("Move forward / backward"),
        ])
        .add_row(vec![
            TableCell::new("Left / Right"),
            TableCell::new("Rotate camera"),
        ])
        .add_row(vec![TableCell::new("Escape"), TableCell::new("Quit")]);
    println!("{controls}\n");
}

/// Build a 16x16 demo map with border walls and interior features.
fn make_demo_map() -> ArrayGridMap {
    let mut map = ArrayGridMap::new(16, 16);

    // Border walls (material 3 = grey).
    for x in 0..16 {
        map.set(x, 0, Cell::Solid(3));
        map.set(x, 15, Cell::Solid(3));
    }
    for y in 0..16 {
        map.set(0, y, Cell::Solid(3));
        map.set(15, y, Cell::Solid(3));
    }

    // Interior rooms.
    for x in 4..8 {
        map.set(x, 4, Cell::Solid(0)); // red wall
    }
    for y in 4..8 {
        map.set(8, y, Cell::Solid(1)); // green wall
    }

    // Pillars / obstacles.
    map.set(10, 10, Cell::Solid(2)); // blue pillar
    map.set(12, 6, Cell::Solid(2));
    map.set(3, 10, Cell::Solid(0));

    map
}

#[allow(clippy::struct_excessive_bools)]
struct RaycastDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    map: ArrayGridMap,
    camera_pos: Vec2Fixed,
    camera_angle: Bam,
    // Input state: which keys are currently held.
    key_forward: bool,
    key_backward: bool,
    key_turn_left: bool,
    key_turn_right: bool,
}

impl RaycastDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|e| HostError::App(e.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT).map_err(|e| HostError::App(e.to_string()))?,
            map: make_demo_map(),
            // Start in the center of the map, facing north.
            camera_pos: Vec2Fixed::from_f32(8.0, 8.0),
            camera_angle: ANG90,
            key_forward: false,
            key_backward: false,
            key_turn_left: false,
            key_turn_right: false,
        })
    }
}

impl WindowApp for RaycastDemoApp {
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

    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        self.framebuffer =
            Framebuffer::new(width, height).map_err(|e| HostError::App(e.to_string()))?;
        self.zbuffer = ZBuffer::new(width, height).map_err(|e| HostError::App(e.to_string()))?;
        Ok(())
    }

    fn input(&mut self, ctx: WindowContext<'_>, event: &WindowEvent) -> Result<(), Self::Error> {
        if let WindowEvent::KeyboardInput {
            event: KeyEvent {
                logical_key, state, ..
            },
            ..
        } = event
        {
            let pressed = *state == ElementState::Pressed;
            match logical_key {
                Key::Character(c) if c.as_str() == "w" => self.key_forward = pressed,
                Key::Character(c) if c.as_str() == "s" => self.key_backward = pressed,
                Key::Named(NamedKey::ArrowLeft) => self.key_turn_left = pressed,
                Key::Named(NamedKey::ArrowRight) => self.key_turn_right = pressed,
                Key::Named(NamedKey::Escape) => {
                    ctx.event_loop.exit();
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Rotation.
        let turn = Bam::from_raw(TURN_SPEED);
        if self.key_turn_left {
            self.camera_angle = self.camera_angle + turn;
        }
        if self.key_turn_right {
            self.camera_angle = self.camera_angle - turn;
        }

        // Movement along the camera direction.
        let speed = Fixed16_16::from_f32(MOVE_SPEED);
        let (sin, cos) = self.camera_angle.sin_cos_fixed();
        let dx = cos.fixed_mul(speed);
        let dy = sin.fixed_mul(speed);

        if self.key_forward {
            let new_x = self.camera_pos.x + dx;
            let new_y = self.camera_pos.y + dy;
            if !self
                .map
                .cell_at(new_x.to_int() as u32, self.camera_pos.y.to_int() as u32)
                .is_solid()
            {
                self.camera_pos.x = new_x;
            }
            if !self
                .map
                .cell_at(self.camera_pos.x.to_int() as u32, new_y.to_int() as u32)
                .is_solid()
            {
                self.camera_pos.y = new_y;
            }
        }
        if self.key_backward {
            let new_x = self.camera_pos.x - dx;
            let new_y = self.camera_pos.y - dy;
            if !self
                .map
                .cell_at(new_x.to_int() as u32, self.camera_pos.y.to_int() as u32)
                .is_solid()
            {
                self.camera_pos.x = new_x;
            }
            if !self
                .map
                .cell_at(self.camera_pos.x.to_int() as u32, new_y.to_int() as u32)
                .is_solid()
            {
                self.camera_pos.y = new_y;
            }
        }

        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Clear to dark background (acts as ceiling/floor color).
        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        // Render the raycasted view.
        render_raycast_view(
            &mut self.framebuffer,
            &mut self.zbuffer,
            &self.map,
            self.camera_pos,
            self.camera_angle,
            ANG90, // 90-degree horizontal FOV
        );

        // Present to screen.
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(&self.framebuffer)?;
        Ok(())
    }
}

fn main() {
    print_banner();
    run_windowed_app(
        RaycastDemoApp::new().map_err(|e| abrash::platform::HostError::App(e.to_string())),
    );
}
