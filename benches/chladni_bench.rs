use abrash_render::experimental::chladni::{ChladniConfig, apply_chladni};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_chladni(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = ChladniConfig::default();

    c.bench_function("apply_chladni_800x600", |b| {
        b.iter(|| apply_chladni(&mut fb, &config));
    });
}

criterion_group!(benches, bench_chladni);
criterion_main!(benches);
