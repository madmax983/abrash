#![cfg(feature = "gpu-engine-compare")]

use abrash::gpu_render::{GpuOffscreenBench, GpuOffscreenBenchConfig};
use abrash::gpu_render::unit_cube_mesh;
use bevy::{
    app::{App, Update},
    ecs::{component::Component, system::Query},
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use fyrox::{
    core::algebra::{Vector2, Vector3},
    scene::{
        Scene, base::BaseBuilder, graph::GraphUpdateSwitches, pivot::PivotBuilder,
        transform::TransformBuilder,
    },
};

#[derive(Component)]
struct BevyParticle {
    position: [f32; 3],
    velocity: [f32; 3],
    energy: f32,
}

fn bevy_particle_update(mut particles: Query<&mut BevyParticle>) {
    for mut particle in &mut particles {
        particle.position[0] += particle.velocity[0];
        particle.position[1] += particle.velocity[1];
        particle.position[2] += particle.velocity[2];
        particle.velocity[1] -= 0.000_98;
        particle.energy *= 0.999_5;

        if particle.position[1] < -2.0 {
            particle.position[1] = 2.0;
            particle.velocity[1] = particle.velocity[1].abs() * 0.95;
        }
    }
}

fn build_bevy_update_app(entity_count: usize) -> App {
    let mut app = App::new();
    app.add_systems(Update, bevy_particle_update);

    let world = app.world_mut();
    for i in 0..entity_count {
        let phase = i as f32 * 0.017_451;
        world.spawn(BevyParticle {
            position: [phase.sin(), phase.cos(), (phase * 0.37).sin()],
            velocity: [
                0.001 + phase.cos().abs() * 0.001_5,
                0.002 + phase.sin().abs() * 0.001_2,
                0.0015 + (phase * 0.5).cos().abs() * 0.001,
            ],
            energy: 1.0,
        });
    }

    app
}

fn bench_wgpu_offscreen_reference(c: &mut Criterion) {
    let (vertices, indices) = unit_cube_mesh();
    let mut group = c.benchmark_group("gpu_engine_compare_wgpu");
    group.sample_size(10);

    let mut bench = match GpuOffscreenBench::new(
        &vertices,
        &indices,
        GpuOffscreenBenchConfig {
            width: 1920,
            height: 1080,
            draw_repeats: 2,
            ..GpuOffscreenBenchConfig::default()
        },
    ) {
        Ok(bench) => bench,
        Err(err) => {
            eprintln!("Skipping wgpu comparison benchmark: {err}");
            return;
        }
    };

    if let Err(err) = bench.render_frames(3) {
        eprintln!("Skipping wgpu comparison benchmark warmup: {err}");
        return;
    }

    group.bench_function("offscreen_1080p_cube_r2", |b| {
        b.iter(|| {
            bench
                .render_frame()
                .expect("wgpu offscreen comparison frame should render");
        });
    });

    group.finish();
}

fn bench_bevy_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_engine_compare_bevy_update");
    group.sample_size(10);

    for &entity_count in &[10_000_usize, 50_000_usize] {
        let mut app = build_bevy_update_app(entity_count);
        for _ in 0..3 {
            app.update();
        }

        group.bench_function(BenchmarkId::new("entities", entity_count), |b| {
            b.iter(|| {
                app.update();
            });
        });
    }

    group.finish();
}

fn build_fyrox_scene(node_count: usize) -> Scene {
    let mut scene = Scene::new();

    for i in 0..node_count {
        let phase = i as f32 * 0.011_327;
        let transform = TransformBuilder::new()
            .with_local_position(Vector3::new(
                phase.sin() * 3.0,
                phase.cos() * 1.5,
                (phase * 0.5).sin() * 2.0,
            ))
            .build();

        let base = BaseBuilder::new()
            .with_name(format!("node_{i}"))
            .with_local_transform(transform);
        PivotBuilder::new(base).build(&mut scene.graph);
    }

    scene
}

fn bench_fyrox_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_engine_compare_fyrox_update");
    group.sample_size(10);
    let frame_size = Vector2::new(1920.0, 1080.0);

    for &node_count in &[10_000_usize, 50_000_usize] {
        let mut scene = build_fyrox_scene(node_count);
        for _ in 0..3 {
            scene.update(frame_size, 1.0 / 60.0, GraphUpdateSwitches::default());
        }

        group.bench_function(BenchmarkId::new("nodes", node_count), |b| {
            b.iter(|| {
                scene.update(frame_size, 1.0 / 60.0, GraphUpdateSwitches::default());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_wgpu_offscreen_reference,
    bench_bevy_update,
    bench_fyrox_update
);
criterion_main!(benches);
