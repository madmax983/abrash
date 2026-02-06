use abrash::{
    framebuffer::Framebuffer,
    hiz_buffer::HiZBuffer,
    math::Vec3,
    tile_renderer::TileRenderer,
    zbuffer::ZBuffer,
};
use std::time::{Duration, Instant};

/// Detailed profiling benchmark to identify SIMD bottlenecks
fn main() {
    println!("=== SIMD Profiling Analysis ===\n");

    // Test different scanline lengths
    profile_scanline_lengths();

    // Test Hi-Z pyramid build
    profile_hiz_pyramid();

    // Test full rendering pipeline
    profile_rendering_pipeline();
}

fn profile_scanline_lengths() {
    println!("## 1. Scanline Length Analysis\n");

    let lengths = [4, 8, 16, 32, 64, 128, 256, 512];

    for &len in &lengths {
        let triangles = generate_horizontal_triangles(len, 100);

        let mut fb = Framebuffer::new(1920, 1080).unwrap();
        let mut zb = ZBuffer::new(1920, 1080).unwrap();
        let mut renderer = TileRenderer::new(1920, 1080);

        // Warmup
        for _ in 0..10 {
            fb.clear(0xFF_00_00_00);
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, &triangles);
        }

        // Measure
        let iterations = 100;
        let start = Instant::now();
        for _ in 0..iterations {
            fb.clear(0xFF_00_00_00);
            zb.clear();
            renderer.render_batch(&mut fb, &mut zb, &triangles);
        }
        let elapsed = start.elapsed();
        let avg_time = elapsed.as_micros() / iterations;

        println!("Scanline length ~{:3} px: {:6} µs/frame", len, avg_time);
    }

    println!();
}

fn profile_hiz_pyramid() {
    println!("## 2. Hi-Z Pyramid Build Profiling\n");

    let resolutions = [
        (800, 600, "800×600"),
        (1920, 1080, "1920×1080"),
        (2560, 1440, "2560×1440"),
        (3840, 2160, "3840×2160"),
    ];

    for &(width, height, name) in &resolutions {
        let zb = ZBuffer::new(width, height).unwrap();
        let mut hiz = HiZBuffer::new(width, height);

        // Warmup
        for _ in 0..10 {
            hiz.build_pyramid(&zb);
        }

        // Measure
        let iterations = 100;
        let start = Instant::now();
        for _ in 0..iterations {
            hiz.build_pyramid(&zb);
        }
        let elapsed = start.elapsed();
        let avg_time = elapsed.as_micros() / iterations;
        let pixels = width * height;
        let ns_per_pixel = (avg_time * 1000) / pixels as u128;

        println!("{}: {:6} µs/build ({:3} ns/pixel)", name, avg_time, ns_per_pixel);
    }

    println!();
}

fn profile_rendering_pipeline() {
    println!("## 3. Full Rendering Pipeline Breakdown\n");

    let triangles = generate_test_scene(100, 1920, 1080);

    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let mut zb = ZBuffer::new(1920, 1080).unwrap();
    let mut renderer = TileRenderer::new(1920, 1080);

    // Warmup
    for _ in 0..10 {
        fb.clear(0xFF_00_00_00);
        zb.clear();
        renderer.render_batch(&mut fb, &mut zb, &triangles);
    }

    // Measure components
    let iterations = 1000;

    // 1. Clear time
    let start = Instant::now();
    for _ in 0..iterations {
        fb.clear(0xFF_00_00_00);
        zb.clear();
    }
    let clear_time = start.elapsed().as_micros() / iterations;

    // 2. Full render time
    let start = Instant::now();
    for _ in 0..iterations {
        fb.clear(0xFF_00_00_00);
        zb.clear();
        renderer.render_batch(&mut fb, &mut zb, &triangles);
    }
    let total_time = start.elapsed().as_micros() / iterations;

    let render_time = total_time - clear_time;

    println!("Clear time:  {:6} µs ({:3}%)", clear_time, (clear_time * 100) / total_time);
    println!("Render time: {:6} µs ({:3}%)", render_time, (render_time * 100) / total_time);
    println!("Total time:  {:6} µs", total_time);

    // Calculate triangles per second
    let tris_per_sec = (triangles.len() as u128 * iterations * 1_000_000) / total_time;
    println!("\nThroughput: {} M triangles/sec", tris_per_sec / 1_000_000);

    println!();
}

fn profile_simd_vs_scalar_detailed() {
    println!("## 4. SIMD vs Scalar Component Breakdown\n");

    // This would require instrumenting the actual code with timing
    // For now, we'll note that this needs internal instrumentation
    println!("Note: Detailed SIMD vs scalar breakdown requires code instrumentation");
    println!("See suggestions below for adding timing to hot paths.\n");
}

/// Generate horizontal triangles of specific scanline length
fn generate_horizontal_triangles(scanline_len: u32, count: usize) -> Vec<((Vec3, f32), (Vec3, f32), (Vec3, f32), u32)> {
    let mut triangles = Vec::new();
    let width = scanline_len as f32;

    for i in 0..count {
        let y = (i as f32) * 10.0;
        let x = 100.0;
        let depth = 5.0 + (i % 5) as f32;

        // Horizontal triangles with specific width
        let v0 = (Vec3::new(x, y, depth), 1.0);
        let v1 = (Vec3::new(x + width, y, depth), 1.0);
        let v2 = (Vec3::new(x + width / 2.0, y + 5.0, depth), 1.0);

        let color = 0xFF_FF_00_00;
        triangles.push((v0, v1, v2, color));
    }

    triangles
}

/// Generate test scene with triangles
fn generate_test_scene(count: usize, width: u32, height: u32) -> Vec<((Vec3, f32), (Vec3, f32), (Vec3, f32), u32)> {
    let mut triangles = Vec::new();

    for i in 0..count {
        let x = ((i % 20) as f32) * (width as f32 / 20.0);
        let y = ((i / 20) as f32) * (height as f32 / 20.0);
        let depth = 5.0 + ((i % 5) as f32) * 2.0;
        let size = 100.0;

        let v0 = (Vec3::new(x, y, depth), 1.0);
        let v1 = (Vec3::new(x + size, y, depth + 0.1), 1.0);
        let v2 = (Vec3::new(x + size/2.0, y + size, depth + 0.2), 1.0);

        let color = 0xFF_00_00_00 | ((i as u32) << 8);
        triangles.push((v0, v1, v2, color));
    }

    triangles
}
