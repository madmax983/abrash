#![cfg(feature = "gpu-render")]

use abrash::gpu_render::{
    GpuDemoConfig, GpuInteractionController, GpuOffscreenBench, GpuOffscreenBenchConfig, GpuVertex,
    unit_cube_mesh,
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn generate_wave_grid_mesh(grid_dim: usize) -> (Vec<GpuVertex>, Vec<u16>) {
    let vertex_count = (grid_dim + 1) * (grid_dim + 1);
    assert!(
        u16::try_from(vertex_count).is_ok(),
        "grid_dim too large for u16 indices: {grid_dim}"
    );

    let mut vertices = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(grid_dim * grid_dim * 6);

    let inv = 1.0 / (grid_dim as f32);

    for y in 0..=grid_dim {
        for x in 0..=grid_dim {
            let fx = x as f32 * inv;
            let fy = y as f32 * inv;

            let px = fx * 3.0 - 1.5;
            let py = fy * 3.0 - 1.5;
            let pz = ((fx * 14.0).sin() * (fy * 11.0).cos()) * 0.22;

            vertices.push(GpuVertex {
                position: [px, py, pz],
                color: [fx, fy, 1.0 - (fx * fy)],
            });
        }
    }

    let row = grid_dim + 1;
    for y in 0..grid_dim {
        for x in 0..grid_dim {
            let i0 = (y * row + x) as u16;
            let i1 = i0 + 1;
            let i2 = ((y + 1) * row + x) as u16;
            let i3 = i2 + 1;

            indices.extend_from_slice(&[i0, i1, i2, i1, i3, i2]);
        }
    }

    (vertices, indices)
}

fn bench_gpu_interaction_controller(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_render_controller");
    let config = GpuDemoConfig::default();
    let mut controller = GpuInteractionController::new(&config);

    group.bench_function("update_60hz", |b| {
        b.iter(|| {
            controller.set_move_right(true);
            controller.set_move_up(true);
            controller.update(1.0 / 60.0, &config);
            controller.set_move_right(false);
            controller.set_move_up(false);
        });
    });

    group.finish();
}

fn bench_gpu_offscreen_frames(c: &mut Criterion) {
    let (vertices, indices) = unit_cube_mesh();
    let mut group = c.benchmark_group("gpu_render_offscreen");
    group.sample_size(10);

    for &(label, width, height) in &[
        ("1280x720", 1280_u32, 720_u32),
        ("1920x1080", 1920_u32, 1080_u32),
    ] {
        let mut bench = match GpuOffscreenBench::new(
            &vertices,
            &indices,
            GpuOffscreenBenchConfig {
                width,
                height,
                ..GpuOffscreenBenchConfig::default()
            },
        ) {
            Ok(bench) => bench,
            Err(err) => {
                eprintln!("Skipping GPU offscreen benchmark ({label}): {err}");
                continue;
            }
        };

        if let Err(err) = bench.render_frames(5) {
            eprintln!("Skipping GPU offscreen benchmark ({label}) during warmup: {err}");
            continue;
        }

        group.bench_function(BenchmarkId::new("frame", label), |b| {
            b.iter(|| {
                bench.render_frame().expect("offscreen frame should render");
            });
        });
    }

    group.finish();
}

fn bench_gpu_offscreen_heavy_frames(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_render_offscreen_heavy");
    group.sample_size(10);

    for &(label, width, height, grid_dim, repeats) in &[
        ("1080p_grid96_r4", 1920_u32, 1080_u32, 96_usize, 4_u32),
        ("1080p_grid160_r8", 1920_u32, 1080_u32, 160_usize, 8_u32),
        ("4k_grid160_r8", 3840_u32, 2160_u32, 160_usize, 8_u32),
    ] {
        let (vertices, indices) = generate_wave_grid_mesh(grid_dim);
        let tri_count = indices.len() / 3;

        let mut bench = match GpuOffscreenBench::new(
            &vertices,
            &indices,
            GpuOffscreenBenchConfig {
                width,
                height,
                draw_repeats: repeats,
                initial_distance: 5.5,
                rotation_speed: 0.35,
                ..GpuOffscreenBenchConfig::default()
            },
        ) {
            Ok(bench) => bench,
            Err(err) => {
                eprintln!("Skipping heavy GPU offscreen benchmark ({label}): {err}");
                continue;
            }
        };

        if let Err(err) = bench.render_frames(3) {
            eprintln!("Skipping heavy GPU offscreen benchmark ({label}) during warmup: {err}");
            continue;
        }

        group.bench_function(
            BenchmarkId::new("frame", format!("{label}_{tri_count}tri")),
            |b| {
                b.iter(|| {
                    bench
                        .render_frame()
                        .expect("heavy offscreen frame should render");
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// CPU vs GPU comparison benchmarks — matched workloads
// ---------------------------------------------------------------------------

/// GPU at production triangle counts to compare against CPU scene_render benchmarks.
/// Uses a single mesh with draw_repeats to approximate multi-object workloads.
/// NOTE: GPU times include ~90µs device.poll() fence overhead.
fn bench_gpu_vs_cpu_workloads(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_vs_cpu");
    group.sample_size(10);

    // grid10 = 200 tris, 121 verts (matches CPU bench mesh)
    let (mesh_sm_v, mesh_sm_i) = generate_wave_grid_mesh(10);
    // grid20 = 800 tris, 441 verts (matches CPU dense mesh)
    let (mesh_md_v, mesh_md_i) = generate_wave_grid_mesh(20);

    for &(label, width, height, verts, indices, repeats) in &[
        // 20K tris @ 1080p (100 × 200 tri mesh) — matches CPU 1080p_20k
        ("1080p_20k_100draws", 1920_u32, 1080_u32, &mesh_sm_v, &mesh_sm_i, 100_u32),
        // 80K tris @ 1080p (100 × 800 tri mesh) — matches CPU 1080p_80k_dense
        ("1080p_80k_100draws", 1920_u32, 1080_u32, &mesh_md_v, &mesh_md_i, 100_u32),
        // 20K tris @ 4K — matches CPU 4k_20k
        ("4k_20k_100draws", 3840_u32, 2160_u32, &mesh_sm_v, &mesh_sm_i, 100_u32),
        // 400K tris @ 1080p (pushing into AAA-lite territory)
        ("1080p_400k_500draws", 1920_u32, 1080_u32, &mesh_md_v, &mesh_md_i, 500_u32),
    ] {
        let tri_count = (indices.len() / 3) * repeats as usize;
        let mut bench = match GpuOffscreenBench::new(
            verts,
            indices,
            GpuOffscreenBenchConfig {
                width,
                height,
                draw_repeats: repeats,
                initial_distance: 5.5,
                rotation_speed: 0.35,
                ..GpuOffscreenBenchConfig::default()
            },
        ) {
            Ok(bench) => bench,
            Err(err) => {
                eprintln!("Skipping GPU vs CPU benchmark ({label}): {err}");
                continue;
            }
        };

        if let Err(err) = bench.render_frames(3) {
            eprintln!("Skipping GPU vs CPU benchmark ({label}) during warmup: {err}");
            continue;
        }

        group.bench_function(
            BenchmarkId::new("frame", format!("{label}_{tri_count}tri")),
            |b| {
                b.iter(|| {
                    bench
                        .render_frame()
                        .expect("GPU vs CPU frame should render");
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_gpu_interaction_controller,
    bench_gpu_offscreen_frames,
    bench_gpu_offscreen_heavy_frames,
    bench_gpu_vs_cpu_workloads
);
criterion_main!(benches);
