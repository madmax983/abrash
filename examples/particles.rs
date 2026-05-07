use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::particles::ParticleSystem;
use abrash::platform::{
    SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};
use abrash::texture::Texture;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use clap::Parser;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::f32::consts::PI;
use std::fmt;
use std::io::Error as IoError;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF10_1010;

#[derive(Debug)]
struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AppError {}

impl From<&'static str> for AppError {
    fn from(error: &'static str) -> Self {
        Self(error.to_string())
    }
}

impl From<String> for AppError {
    fn from(error: String) -> Self {
        Self(error)
    }
}

impl From<IoError> for AppError {
    fn from(error: IoError) -> Self {
        Self(error.to_string())
    }
}

impl From<abrash::platform::HostError> for AppError {
    fn from(error: abrash::platform::HostError) -> Self {
        Self(error.to_string())
    }
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Nova Particle System Demo",
    long_about = None
)]
struct Args {
    /// Number of particles
    #[arg(short, long, default_value_t = 1000)]
    count: usize,

    /// Emission rate (particles per second)
    #[arg(short, long, default_value_t = 50.0)]
    rate: f32,

    /// Spread factor
    #[arg(short, long, default_value_t = 0.8)]
    spread: f32,

    /// Start life duration (seconds)
    #[arg(long, default_value_t = 2.0)]
    start_life: f32,

    /// Start size
    #[arg(long, default_value_t = 0.5)]
    start_size: f32,
}

fn create_particle_texture() -> Texture {
    let size = 32;
    let mut tex = Texture::new(size, size).unwrap();
    let center = size as f32 / 2.0;
    let max_dist = center;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            #[allow(clippy::imprecise_flops)]
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > max_dist {
                tex.set_pixel(x, y, 0x0000_0000);
            } else {
                let t = dist / max_dist;
                let alpha = 1.0 + t * 253.0;
                let alpha_u8 = alpha as u8;

                let r = 255;
                let g = ((1.0 - t) * 200.0) as u8;
                let b = 0;

                let color =
                    (u32::from(alpha_u8) << 24) | ((r as u32) << 16) | (u32::from(g) << 8) | b;
                tex.set_pixel(x, y, color);
            }
        }
    }
    tex.generate_mipmaps();
    tex
}

fn print_banner(args: &Args) {
    println!("\n{}", "✨ Nova Particle System".bold().cyan());
    println!("{}", "=====================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Parameter").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Particle Count"),
            Cell::new(args.count.to_string()).fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Emission Rate"),
            Cell::new(args.rate.to_string()).fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Spread"),
            Cell::new(args.spread.to_string()),
        ])
        .add_row(vec![
            Cell::new("Start Life"),
            Cell::new(format!("{:.1}s", args.start_life)),
        ])
        .add_row(vec![
            Cell::new("Start Size"),
            Cell::new(args.start_size.to_string()),
        ]);

    println!("\n{}", "⚙️  Configuration".bold());
    println!("{table}");

    println!("\n{}", "🎮 Controls".bold());
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Close Window"),
            Cell::new("Exit Application"),
        ])
        .add_row(vec![
            Cell::new("Interactive"),
            Cell::new("(Coming Soon)").fg(Color::DarkGrey),
        ]);
    println!("{controls}\n");
}

struct ParticleDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    timestep: FixedTimestep,
    particles: ParticleSystem,
    projection: Mat4,
    angle: f32,
}

impl ParticleDemoApp {
    fn new(args: &Args) -> Result<Self, AppError> {
        let texture = create_particle_texture();
        let mut particles = ParticleSystem::new(args.count, texture);
        particles.emission_rate = args.rate;
        particles.start_life = args.start_life;
        particles.spread = args.spread;
        particles.start_size = args.start_size;

        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)?,
            timestep: FixedTimestep::new(60),
            particles,
            projection: Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0),
            angle: 0.0,
        })
    }

    fn present(&mut self) -> Result<(), AppError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| IoError::other("software presenter not initialized"))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

impl WindowApp for ParticleDemoApp {
    type Error = AppError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Nova - Particle System".to_string(),
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
        let steps = self.timestep.update();
        for _ in 0..steps {
            self.angle += 0.5 * self.timestep.dt();
            self.particles.update(self.timestep.dt());
        }
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.framebuffer.clear(BACKGROUND);
        self.zbuffer.clear();

        let eye = Vec3::new(self.angle.sin() * 5.0, 2.0, self.angle.cos() * 5.0);
        let target = Vec3::new(0.0, 1.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);

        self.particles.render(
            &mut self.framebuffer,
            &mut self.zbuffer,
            view,
            self.projection,
        );

        self.present()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), AppError> {
    let args = Args::parse();
    print_banner(&args);
    run_windowed(ParticleDemoApp::new(&args).unwrap());
    Ok(())
}
