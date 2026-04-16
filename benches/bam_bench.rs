use abrash_core::bam::{ANG45, Bam};
use abrash_core::math::fast_sin_cos;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bam_table_lookup(c: &mut Criterion) {
    let angle = ANG45;
    c.bench_function("bam_sin_cos_fixed (table)", |b| {
        b.iter(|| black_box(black_box(angle).sin_cos_fixed()));
    });
}

fn bam_polynomial(c: &mut Criterion) {
    let angle = ANG45;
    c.bench_function("bam_sin_cos_f32 (polynomial)", |b| {
        b.iter(|| black_box(black_box(angle).sin_cos_f32()));
    });
}

fn std_sin_cos(c: &mut Criterion) {
    let rad = std::f32::consts::FRAC_PI_4;
    c.bench_function("f32::sin_cos (stdlib)", |b| {
        b.iter(|| black_box(black_box(rad).sin_cos()));
    });
}

fn batch_1000_table(c: &mut Criterion) {
    let angles: Vec<Bam> = (0..1000).map(|i| Bam(i * 4_294_967)).collect();
    c.bench_function("1000x bam_sin_cos_fixed", |b| {
        b.iter(|| {
            for &a in &angles {
                black_box(a.sin_cos_fixed());
            }
        });
    });
}

fn batch_1000_polynomial(c: &mut Criterion) {
    let angles: Vec<f32> = (0..1000)
        .map(|i| i as f32 * std::f32::consts::TAU / 1000.0)
        .collect();
    c.bench_function("1000x f32::sin_cos (polynomial)", |b| {
        b.iter(|| {
            for &a in &angles {
                black_box(f32::sin_cos(a));
            }
        });
    });
}

criterion_group!(
    benches,
    bam_table_lookup,
    bam_polynomial,
    std_sin_cos,
    batch_1000_table,
    batch_1000_polynomial
);
criterion_main!(benches);
