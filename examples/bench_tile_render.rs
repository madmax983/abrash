use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::{ClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use std::time::Instant;

use clap::Parser;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;

#[derive(Parser, Debug)]
#[command(author, version, about = "TileRenderer Benchmark", long_about = None)]
struct Args {
    /// Number of triangles
    #[arg(short, long, default_value_t = 500)]
    triangles: usize,

    /// Framebuffer width
    #[arg(long, default_value_t = 3840)]
    width: u32,

    /// Framebuffer height
    #[arg(long, default_value_t = 2160)]
    height: u32,

    /// Number of iterations
    #[arg(short, long, default_value_t = 20)]
    iterations: usize,
}

fn generate_overlapping_triangles(count: usize) -> Vec<ClipTriangle> {
    let mut tris = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f32 * 0.1;
        let offset_x = (t * 1.7).sin() * 0.15;
        let offset_y = (t * 2.3).cos() * 0.15;
        let depth = 3.0 + (t * 0.5).sin() * 2.0;

        // Use w = depth to match the criterion benchmark
        let w = depth;
        let v0 = (Vec3::new(offset_x, 0.5 + offset_y, depth), w);
        let v1 = (Vec3::new(-0.5 + offset_x, -0.5 + offset_y, depth), w);
        let v2 = (Vec3::new(0.5 + offset_x, -0.5 + offset_y, depth), w);

        let r = ((i * 37) % 256) as u32;
        let g = ((i * 73) % 256) as u32;
        let b = ((i * 113) % 256) as u32;
        let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

        tris.push((v0, v1, v2, color));
    }
    tris
}

fn print_banner(width: u32, height: u32, triangle_count: usize) {
    println!("\n{}", "🚀 TileRenderer Benchmark".bold().cyan());
    println!("{}", "=========================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Parameter").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Resolution"),
            Cell::new(format!("{width}x{height}")).fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Triangles"),
            Cell::new(triangle_count.to_string()).fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Configuration".bold());
    println!("{table}");
}

fn main() {
    let args = Args::parse();

    let mut fb = Framebuffer::new(args.width, args.height).unwrap();
    let mut zb = ZBuffer::new(args.width, args.height).unwrap();
    let mut tr = TileRenderer::new(args.width, args.height);

    print_banner(args.width, args.height, args.triangles);

    let triangles = generate_overlapping_triangles(args.triangles);

    println!("\n{}", "🔥 Warming up (10 frames)...".yellow());
    // Warmup
    for _ in 0..10 {
        zb.clear();
        tr.render_batch(&mut fb, &mut zb, &triangles);
    }

    println!("⏱️  Running benchmark ({} frames)...", args.iterations);
    let start = Instant::now();
    for _ in 0..args.iterations {
        zb.clear();
        tr.render_batch(&mut fb, &mut zb, &triangles);
    }
    let duration = start.elapsed();
    let avg_time = duration.as_secs_f64() * 1000.0 / (args.iterations as f64);

    let mut results = Table::new();
    results
        .load_preset(presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_header(vec![
            Cell::new("Metric").fg(Color::Cyan),
            Cell::new("Result").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Total Time"),
            Cell::new(format!("{:.4} s", duration.as_secs_f64())),
        ])
        .add_row(vec![
            Cell::new("Avg Time / Frame").add_attribute(comfy_table::Attribute::Bold),
            Cell::new(format!("{avg_time:.4} ms"))
                .fg(Color::Green)
                .add_attribute(comfy_table::Attribute::Bold),
        ]);

    println!("\n{}", "📊 Results".bold());
    println!("{results}\n");
}
