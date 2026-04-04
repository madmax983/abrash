use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::duotone::apply_duotone;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_duotone(c: &mut Criterion) {
    let mut group = c.benchmark_group("duotone");

    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with some dummy data
    for (i, pixel) in fb.as_mut_slice().iter_mut().enumerate() {
        *pixel = 0xFF00_0000 | (i as u32 & 0x00FF_FFFF);
    }

    let color_dark = 0xFF000040;
    let color_light = 0xFFFFCC00;

    group.bench_function("1080p", |b| {
        b.iter(|| {
            apply_duotone(
                black_box(&mut fb),
                black_box(color_dark),
                black_box(color_light),
            );
        })
    });

    group.finish();
}

criterion_group!(benches, bench_duotone);
criterion_main!(benches);
