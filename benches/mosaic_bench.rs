use abrash::framebuffer::Framebuffer;
use abrash::experimental::mosaic::apply_hex_mosaic;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_mosaic(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFFFF_FFFF);

    c.bench_function("apply_hex_mosaic (800x600)", |b| {
        b.iter(|| apply_hex_mosaic(&mut fb, 10.0, 1.0, 0xFF00_0000))
    });
}

criterion_group!(benches, bench_mosaic);
criterion_main!(benches);
