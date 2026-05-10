#![allow(clippy::missing_panics_doc)]
use abrash::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use abrash_render::experimental::metaballs::{Metaball, render_metaballs};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn metaballs_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let metaballs = vec![
        Metaball {
            position: Vec2::new(400.0, 300.0),
            radius: 150.0,
            color: 0xFFFF_0000,
        },
        Metaball {
            position: Vec2::new(300.0, 400.0),
            radius: 100.0,
            color: 0xFF00_FF00,
        },
        Metaball {
            position: Vec2::new(500.0, 200.0),
            radius: 120.0,
            color: 0xFF00_00FF,
        },
    ];

    c.bench_function("render_metaballs 800x600 3_balls", |b| {
        b.iter(|| {
            fb.clear(0xFF00_0000);
            render_metaballs(black_box(&mut fb), black_box(&metaballs), black_box(1.0));
        });
    });
}

criterion_group!(benches, metaballs_benchmark);
criterion_main!(benches);
