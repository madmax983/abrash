#![cfg(feature = "gpu-render")]

use abrash::gpu_render::{
    GpuDemoConfig, GpuInteractionController, GpuOffscreenBench, GpuOffscreenBenchConfig,
    unit_cube_mesh,
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

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

criterion_group!(
    benches,
    bench_gpu_interaction_controller,
    bench_gpu_offscreen_frames
);
criterion_main!(benches);
