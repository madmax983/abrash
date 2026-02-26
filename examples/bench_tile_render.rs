use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::{ClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use clap::Parser;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::io::{Write, stdout};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Benchmark for TileRenderer",
    long_about = "Benchmarks the software tile rasterizer performance with configurable parameters."
)]
struct Args {
    /// Width of the framebuffer
    #[arg(short = 'W', long, default_value_t = 3840)]
    width: u32,

    /// Height of the framebuffer
    #[arg(short = 'H', long, default_value_t = 2160)]
    height: u32,

    /// Number of triangles to render
    #[arg(short, long, default_value_t = 500)]
    triangles: usize,

    /// Number of benchmark iterations (warmup is separate)
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

fn print_banner(args: &Args) {
    println!("\n{}", "🚀 TileRenderer Benchmark".bold().cyan());
    println!("{}", "=========================".dark_grey());

    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Parameter").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Resolution"),
            Cell::new(format!("{}x{}", args.width, args.height)).fg(Color::Green),
        ])
        .add_row(vec![
            Cell::new("Triangles"),
            Cell::new(args.triangles.to_string()).fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Iterations"),
            Cell::new(args.iterations.to_string()),
        ]);

    println!("\n{}", "⚙️  Configuration".bold());
    println!("{table}");
    println!();
}

fn main() {
    let args = Args::parse();

    if args.width == 0 || args.height == 0 {
        eprintln!("{}", "Error: Width and Height must be non-zero.".red());
        std::process::exit(1);
    }
    if args.iterations == 0 {
        eprintln!("{}", "Error: Iterations must be at least 1.".red());
        std::process::exit(1);
    }

    print_banner(&args);

    let mut fb = Framebuffer::new(args.width, args.height).unwrap();
    let mut zb = ZBuffer::new(args.width, args.height).unwrap();
    let mut tr = TileRenderer::new(args.width, args.height);

    let triangles = generate_overlapping_triangles(args.triangles);

    // Warmup
    print!("🔥 Warming up (10 iters)... ");
    let _ = stdout().flush();
    for _ in 0..10 {
        zb.clear();
        tr.render_batch(&mut fb, &mut zb, &triangles);
    }
    println!("{}", "Done".green());

    println!("⏱️  Running benchmark...");
    let start = Instant::now();
    for i in 0..args.iterations {
        zb.clear();
        tr.render_batch(&mut fb, &mut zb, &triangles);

        // Simple progress bar
        print!("\r   Progress: [{: <20}] {}/{}",
            "=".repeat(((i + 1) * 20 / args.iterations) as usize),
            i + 1,
            args.iterations
        );
        let _ = stdout().flush();
    }
    println!(); // Newline after progress

    let duration = start.elapsed();
    let avg_ms = duration.as_secs_f64() * 1000.0 / args.iterations as f64;
    let total_pixels = (args.width * args.height) as f64;
    let fill_rate_mp = (total_pixels * args.iterations as f64) / duration.as_secs_f64() / 1_000_000.0; // MegaPixels/sec (very rough estimate assuming full overdraw/coverage)

    let mut result_table = Table::new();
    result_table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Metric").fg(Color::Cyan),
            Cell::new("Result").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Total Time"),
            Cell::new(format!("{:.2?}", duration)),
        ])
        .add_row(vec![
            Cell::new("Avg Time / Frame"),
            Cell::new(format!("{:.4} ms", avg_ms)).fg(Color::Green).add_attribute(comfy_table::Attribute::Bold),
        ])
        .add_row(vec![
            Cell::new("Screen Bandwidth"),
            // Note: This is theoretical fill rate based on screen size, not actual drawn pixels
            Cell::new(format!("{:.2} MP/s (screen)", fill_rate_mp)).fg(Color::Magenta),
        ]);

    println!("\n{}", "📊 Results".bold());
    println!("{result_table}\n");
}
