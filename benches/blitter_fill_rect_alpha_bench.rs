use abrash_core::blitter::fill_rect_alpha;
use abrash_core::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_fill_rect_alpha(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let mut group = c.benchmark_group("fill_rect_alpha_bench");

    let sizes = [16, 32, 64, 128, 256];
    for &s in sizes.iter() {
        group.throughput(criterion::Throughput::Elements((s * s) as u64));
        group.bench_with_input(criterion::BenchmarkId::from_parameter(s), &s, |b, &s| {
            let color = 0x80FF0000;
            b.iter(|| {
                fill_rect_alpha(
                    black_box(&mut fb),
                    black_box(100),
                    black_box(100),
                    black_box(s),
                    black_box(s),
                    black_box(color),
                );
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_fill_rect_alpha);
criterion_main!(benches);
