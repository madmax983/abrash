#[cfg(feature = "nova")]
use abrash::experimental::fire::{FireEffect, render_fire};
use abrash::framebuffer::Framebuffer;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[cfg(feature = "nova")]
fn bench_fire_effect(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut fire = FireEffect::new(width, height);

    // Setup initial fire
    for _ in 0..10 {
        fire.seed_bottom_row();
        fire.update();
    }

    c.bench_function("fire_update_640x480", |b| {
        b.iter(|| {
            fire.seed_bottom_row();
            fire.update();
            black_box(fire.buffer());
        })
    });

    c.bench_function("fire_render_640x480", |b| {
        b.iter(|| {
            render_fire(black_box(&mut fb), black_box(&fire));
        })
    });
}

#[cfg(not(feature = "nova"))]
fn bench_fire_effect(_c: &mut Criterion) {}

criterion_group!(benches, bench_fire_effect);
criterion_main!(benches);
