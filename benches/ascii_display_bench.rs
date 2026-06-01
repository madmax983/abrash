use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::ascii_display::{AsciiDisplayConfig, apply_ascii_display};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_ascii_display(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let config = AsciiDisplayConfig::default();

    c.bench_function("ascii_display_1080p", |b| {
        b.iter(|| {
            apply_ascii_display(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_ascii_display);
criterion_main!(benches);
