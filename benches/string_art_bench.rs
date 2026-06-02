use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::string_art::apply_string_art;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_string_art(c: &mut Criterion) {
    let mut group = c.benchmark_group("String Art Filter");

    // Standard high-res config
    let width = 800;
    let height = 600;

    group.bench_function("string_art_800x600_200p_2000l", |b| {
        b.iter_batched(
            || {
                let mut fb = Framebuffer::new(width, height).unwrap();
                // Simple pattern
                fb.clear(0xFFFF_FFFF);
                fb.set_pixel((width / 2) as i32, (height / 2) as i32, 0xFF00_0000);
                fb
            },
            |mut fb| {
                apply_string_art(
                    black_box(&mut fb),
                    black_box(200),
                    black_box(2000),
                    black_box(0.1),
                );
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_string_art);
criterion_main!(benches);
