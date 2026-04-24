//! Physarum Simulation Demo
//!
//! An interactive demonstration of the Physarum polycephalum slime mold simulation.

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::physarum::{PhysarumSim, render_physarum};

#[cfg(not(feature = "nova"))]
fn main() {
    println!("Please run this demo with the `nova` feature enabled:");
    println!("cargo run --release --example physarum_demo --features nova");
}

#[cfg(feature = "nova")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 800;
    let height = 600;

    let mut sim = PhysarumSim::new(width, height);

    // Seed initial agents in a circle
    use std::f32::consts::PI;
    let num_agents = 5000;
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let radius = 100.0;

    for i in 0..num_agents {
        let angle = (i as f32 / num_agents as f32) * PI * 2.0;
        let x = center_x + angle.cos() * radius;
        let y = center_y + angle.sin() * radius;

        // Agents face outward
        sim.add_agent(x, y, angle);
    }

    let mut fb = Framebuffer::new(width, height)?;

    println!("Simulating Physarum...");
    for _ in 0..100 {
        sim.step();
    }

    render_physarum(&sim, &mut fb);

    println!("Simulation complete. 100 steps run.");

    Ok(())
}
