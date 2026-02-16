use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::particles::ParticleSystem;
use abrash::platform::{Window, WindowBackend};
use abrash::texture::Texture;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use clap::Parser;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF10_1010;

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
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > max_dist {
                // Fully transparent
                // In Abrash engine (currently), 0 is skipped (transparent),
                // but 254 is also transparent in blending path.
                tex.set_pixel(x as u32, y as u32, 0x0000_0000);
            } else {
                // Smooth falloff
                let t = dist / max_dist; // 0.0 (center) to 1.0 (edge)

                // We want Center = Opaque, Edge = Transparent.
                // Abrash blending quirk:
                // Alpha=1 -> Opaque (Src * 254 + Dest * 1)
                // Alpha=254 -> Transparent (Src * 1 + Dest * 254)
                // Alpha=255 -> Opaque (Overwrite)

                // So we map t (0..1) to Alpha (1..254)
                let alpha = 1.0 + t * 253.0;
                let alpha_u8 = alpha as u8;

                // Color: Orange Fire
                let r = 255;
                let g = ((1.0 - t) * 200.0) as u8; // Redder at edge
                let b = 0;

                let color = ((alpha_u8 as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b;
                tex.set_pixel(x as u32, y as u32, color);
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
    println!(" • Close window to exit");
    println!(" • (Interactive controls coming soon)\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    print_banner(&args);

    let mut window = Window::new("Nova - Particle System", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    let texture = create_particle_texture();
    let mut particles = ParticleSystem::new(args.count, texture);
    particles.emission_rate = args.rate;
    particles.start_life = args.start_life;
    particles.spread = args.spread;
    particles.start_size = args.start_size;

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);

    // Rotate camera around center
    let mut angle: f32 = 0.0;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle += 0.5 * timestep.dt();
            particles.update(timestep.dt());
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        let eye = Vec3::new(angle.sin() * 5.0, 2.0, angle.cos() * 5.0);
        let target = Vec3::new(0.0, 1.0, 0.0); // Look slightly up
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);

        // Draw grid floor (optional, for reference)
        // ...

        particles.render(&mut framebuffer, &mut zbuffer, view, projection);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
