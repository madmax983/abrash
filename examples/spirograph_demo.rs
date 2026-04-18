use abrash::experimental::spirograph::{SpirographConfig, draw_spirograph};
use abrash_core::framebuffer::Framebuffer;
use std::time::Instant;

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    fb.clear(0xFF_000000);

    // Draw some cool hypotrochoids/epitrochoids
    let configs = vec![
        SpirographConfig {
            fixed_radius: 150.0,
            moving_radius: 52.0,
            pen_offset: 75.0,
            color: 0xFF_FF5555,
            resolution: 200,
            rotations: 52,
            center_x: 200,
            center_y: 200,
        },
        SpirographConfig {
            fixed_radius: 100.0,
            moving_radius: -24.0, // epitrochoid
            pen_offset: 50.0,
            color: 0xFF_55FF55,
            resolution: 200,
            rotations: 24,
            center_x: 600,
            center_y: 200,
        },
        SpirographConfig {
            fixed_radius: 120.0,
            moving_radius: 10.0,
            pen_offset: 120.0,
            color: 0xFF_5555FF,
            resolution: 200,
            rotations: 10,
            center_x: 400,
            center_y: 400,
        },
    ];

    println!("Generating Spirographs...");
    let start = Instant::now();
    for config in &configs {
        draw_spirograph(&mut fb, config);
    }
    let duration = start.elapsed();

    println!("Spirographs generated in {duration:?}");

    // Save output
    let path = "spirograph_output.ppm";
    if let Err(e) = fb.export_ppm(path) {
        eprintln!("Failed to export ppm: {e}");
    } else {
        println!("Exported result to {path}");
    }

    println!("Example complete.");
}
