use abrash::experimental::voxelizer::voxelize_mesh;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{Window, WindowBackend};
use abrash::procedural;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use clap::Parser;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF101010;

#[derive(Parser, Debug)]
#[command(author, version, about = "Nova Explosion Demo")]
struct Args {
    #[arg(short, long, default_value_t = 10000)]
    count: usize,
    #[arg(short, long, default_value_t = 2.0)]
    speed: f32,
}

fn print_banner(args: &Args) {
    println!("\n{}", "✨ Nova Voxel Explosion".bold().cyan());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Parameter").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Particle Count"),
            Cell::new(args.count.to_string()),
        ])
        .add_row(vec![
            Cell::new("Explosion Speed"),
            Cell::new(args.speed.to_string()),
        ]);

    println!("{table}");
    println!("\nControls: Auto-looping animation (Wait for it...)");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    print_banner(&args);

    let mut window = Window::new("Nova - Voxel Explosion", WIDTH, HEIGHT)?;
    let mut fb = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zb = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    // 1. Create Texture (Plasma)
    let texture =
        procedural::plasma(64, 64).map_err(|e| format!("Failed to create texture: {}", e))?;

    // 2. Create Mesh (Cube)
    let mesh = Mesh::cube(2.0);

    // 3. Voxelize
    println!("Voxelizing mesh into {} particles...", args.count);
    // voxelize_mesh consumes texture
    let mut particles = voxelize_mesh(&mesh, args.count, texture);
    particles.start_size = 0.05;

    // Store original positions for reset
    let original_positions: Vec<Vec3> = particles.particles.iter().map(|p| p.position).collect();

    // Camera
    let proj = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let mut angle = 0.0f32;

    // Animation state
    let mut timer = 0.0f32;
    enum State {
        Idle,
        Exploding,
        Resetting,
    }
    let mut state = State::Idle;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            let dt = timestep.dt();
            angle += 0.5 * dt;
            timer += dt;

            match state {
                State::Idle => {
                    // Spin while idle
                    if timer > 2.0 {
                        state = State::Exploding;
                        timer = 0.0;
                        // Apply explosion velocity
                        for p in &mut particles.particles {
                            // Cube centered at 0,0,0
                            let dir = p.position.normalize();
                            // Randomize speed slightly would be nice, but uniform is cool too
                            p.velocity = dir * args.speed;
                        }
                    }
                }
                State::Exploding => {
                    particles.update(dt);
                    if timer > 3.0 {
                        state = State::Resetting;
                        timer = 0.0;
                    }
                }
                State::Resetting => {
                    // Reset positions and velocity
                    // Note: We need to make sure we don't lose particles due to life expiration.
                    // voxelize_mesh sets life to 10000.0.
                    // But update() reduces life.
                    // We should reset life too.
                    for (i, p) in particles.particles.iter_mut().enumerate() {
                        if i < original_positions.len() {
                            p.position = original_positions[i];
                            p.velocity = Vec3::new(0.0, 0.0, 0.0);
                            p.life = 10000.0;
                        }
                    }
                    // Handle case where some particles might have been removed?
                    // ParticleSystem::update removes dead particles.
                    // If max_life is 10000 and we run for 3s, they won't die.

                    state = State::Idle;
                    timer = 0.0;
                }
            }
        }

        fb.clear(BACKGROUND);
        zb.clear();

        let eye = Vec3::new(angle.sin() * 5.0, 3.0, angle.cos() * 5.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);

        particles.render(&mut fb, &mut zb, view, proj);
        window.blit_framebuffer(&fb);
    }

    Ok(())
}
