//! Benchmark binary for `FilmGrain` post-processing.
//!
//! Provides a baseline metric for iterating on pixel shader effects.

use abrash::framebuffer::Framebuffer;
use abrash::post_process::filters::{FilmGrainConfig, apply_film_grain};
use comfy_table::{
    Attribute, Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
};
use std::time::Instant;

fn main() {
    println!("\n🎨 Mosaic Profiler: Film Grain Benchmark\n");

    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    fb.clear(0xFF80_8080);
    let config = FilmGrainConfig {
        intensity: 0.25,
        seed: 0x0BAD_F00D,
    };

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        apply_film_grain(&mut fb, &config);
    }
    let duration = start.elapsed();
    let avg_duration = duration / iterations;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("🎬 Metric").add_attribute(Attribute::Bold),
            Cell::new("Value").add_attribute(Attribute::Bold),
        ]);

    table.add_row(vec![Cell::new("Resolution"), Cell::new("1920x1080")]);
    table.add_row(vec![Cell::new("Iterations"), Cell::new(iterations)]);
    table.add_row(vec![
        Cell::new("Total Time").fg(Color::Cyan),
        Cell::new(format!("{duration:?}")).fg(Color::Cyan),
    ]);
    table.add_row(vec![
        Cell::new("Avg Time/Iteration")
            .fg(Color::Green)
            .add_attribute(Attribute::Bold),
        Cell::new(format!("{avg_duration:?}"))
            .fg(Color::Green)
            .add_attribute(Attribute::Bold),
    ]);

    println!("{table}\n");
}
