//! Physarum (Slime Mold) Simulation Demo.
//!
//! Visualizes the emergent patterns created by thousands of simulated agents.

use abrash::experimental::physarum::{
    PhysarumAgent, PhysarumConfig, diffuse_and_evaporate, render_physarum, update_agents,
};
use abrash::framebuffer::Framebuffer;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;
const NUM_AGENTS: usize = 100_000;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("🌟 Nova: Physarum Simulation")
            .with_inner_size(LogicalSize::new(WIDTH as f64, HEIGHT as f64))
            .build(&event_loop)
            .unwrap(),
    );

    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = Surface::new(&context, window.clone()).unwrap();

    let mut fb = Framebuffer::new(WIDTH as u32, HEIGHT as u32).unwrap();

    let mut trail_map_a = vec![0.0; WIDTH * HEIGHT];
    let mut trail_map_b = vec![0.0; WIDTH * HEIGHT];

    // Spawn agents in a circle
    let mut agents = Vec::with_capacity(NUM_AGENTS);
    let center_x = WIDTH as f32 / 2.0;
    let center_y = HEIGHT as f32 / 2.0;

    for i in 0..NUM_AGENTS {
        let angle = (i as f32 / NUM_AGENTS as f32) * std::f32::consts::TAU;
        let radius = 100.0 + ((i % 100) as f32); // Give it some spread

        let x = center_x + angle.cos() * radius;
        let y = center_y + angle.sin() * radius;

        // Face inward
        let heading = angle + std::f32::consts::PI;

        agents.push(PhysarumAgent::new(x, y, heading));
    }

    let config = PhysarumConfig::default();
    let mut frame_count = 0;

    event_loop
        .run(move |event, elwt| {
            if let Event::WindowEvent { event, .. } = event {
                if matches!(event, WindowEvent::CloseRequested) {
                    elwt.exit();
                }
            }

            if let Event::AboutToWait = event {
                // Determine active buffers
                let (current_trail, next_trail) = if frame_count % 2 == 0 {
                    (&mut trail_map_a, &mut trail_map_b)
                } else {
                    (&mut trail_map_b, &mut trail_map_a)
                };

                // Update
                update_agents(
                    &mut agents,
                    current_trail,
                    WIDTH,
                    HEIGHT,
                    &config,
                    frame_count as u32,
                );
                diffuse_and_evaporate(current_trail, next_trail, WIDTH, HEIGHT, &config);

                // Render
                render_physarum(next_trail, &mut fb, 0xFF000000, 0xFF00FFFF); // Black to Cyan

                // Draw to screen
                if frame_count == 0 {
                    surface
                        .resize(
                            NonZeroU32::new(WIDTH as u32).unwrap(),
                            NonZeroU32::new(HEIGHT as u32).unwrap(),
                        )
                        .unwrap();
                }

                let mut buffer = surface.buffer_mut().unwrap();
                buffer.copy_from_slice(fb.as_slice());
                buffer.present().unwrap();

                frame_count += 1;
            }
        })
        .unwrap();
}
