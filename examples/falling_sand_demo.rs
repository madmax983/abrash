use abrash::experimental::falling_sand::{FallingSandSimulation, ParticleType};
use abrash::framebuffer::Framebuffer;
use std::time::{Duration, Instant};

const WIDTH: usize = 320;
const HEIGHT: usize = 200;

fn main() {
    println!("🌟 Nova: Falling Sand Simulation Demo starting...");

    let mut fb = Framebuffer::new(WIDTH as u32, HEIGHT as u32).unwrap();
    let mut sim = FallingSandSimulation::new(WIDTH, HEIGHT, 12345);

    // Create a wood platform
    for x in 100..200 {
        sim.set_particle(x, 150, ParticleType::Wood, 0xFF_8B4513);
    }
    for x in 120..180 {
        sim.set_particle(x, 170, ParticleType::Wood, 0xFF_8B4513);
    }
    sim.set_particle(100, 140, ParticleType::Wood, 0xFF_8B4513);
    sim.set_particle(199, 140, ParticleType::Wood, 0xFF_8B4513);


    let mut frame_count = 0;

    // Simulate some frames
    while frame_count < 300 {
        // Emit sand
        if frame_count % 2 == 0 {
            sim.set_particle(
                140 + (frame_count % 20),
                10,
                ParticleType::Sand,
                0xFF_EDC9AF,
            );
            sim.set_particle(
                160 - (frame_count % 10),
                15,
                ParticleType::Sand,
                0xFF_F4A460,
            );
        }
        // Emit water
        if frame_count > 100 && frame_count % 3 == 0 {
            sim.set_particle(150, 5, ParticleType::Water, 0xFF_1E90FF);
        }

        sim.update();
        frame_count += 1;
    }

    // Render to framebuffer
    fb.clear(0xFF_000000);
    sim.apply(&mut fb);

    println!("Simulation finished. {} frames computed.", frame_count);
}
