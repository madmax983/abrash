use abrash_core::color::Color;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_color_blend(c: &mut Criterion) {
    let src_u32 = 0x80FF_0000; // Semi-transparent Red
    let dst_u32 = 0xFF00_FF00; // Opaque Green

    let src = Color::from_argb_u32(src_u32);
    let dst = Color::from_argb_u32(dst_u32);

    let mut group = c.benchmark_group("Color_Blending");

    group.bench_function("f32_blend_over", |b| {
        b.iter(|| {
            let res = Color::blend_over(black_box(src), black_box(dst));
            black_box(res.to_argb_u32());
        });
    });

    group.bench_function("u32_swar_blend", |b| {
        b.iter(|| {
            let res = Color::blend_over_u32_swar(black_box(src_u32), black_box(dst_u32));
            black_box(res);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_color_blend);
criterion_main!(benches);
