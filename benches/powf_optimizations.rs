use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_powf_optimizations(c: &mut Criterion) {
    c.bench_function("henyey_greenstein_powf", |b| {
        b.iter(|| {
            let cos_theta = black_box(0.5_f32);
            let g = black_box(0.5_f32);
            use std::f32::consts::PI;
            let g2 = g * g;
            let denom = (1.0 + g2 - 2.0 * g * cos_theta).max(0.0).powf(1.5);
            black_box((1.0 - g2) / (4.0 * PI * denom.max(1e-10)))
        })
    });

    c.bench_function("henyey_greenstein_sqrt", |b| {
        b.iter(|| {
            let cos_theta = black_box(0.5_f32);
            let g = black_box(0.5_f32);
            use std::f32::consts::PI;
            let g2 = g * g;
            let base = (1.0 + g2 - 2.0 * g * cos_theta).max(0.0);
            let denom = base * base.sqrt();
            black_box((1.0 - g2) / (4.0 * PI * denom.max(1e-10)))
        })
    });

    c.bench_function("isosurface_powf", |b| {
        b.iter(|| {
            let total_cells = black_box(1000.0_f32);
            let estimated_vertices = total_cells.powf(0.666_666_7) as usize * 3;
            black_box(estimated_vertices)
        })
    });

    c.bench_function("isosurface_cbrt", |b| {
        b.iter(|| {
            let total_cells = black_box(1000.0_f32);
            let estimated_vertices = total_cells.cbrt().powi(2) as usize * 3;
            black_box(estimated_vertices)
        })
    });

    c.bench_function("sdf_powf_two_thirds", |b| {
        b.iter(|| {
            let p_x = black_box(0.5_f32);
            let k = black_box(2.0_f32);
            let r = (k * p_x * 0.5).powf(2.0_f32 / 3.0);
            black_box(r)
        })
    });

    c.bench_function("sdf_cbrt_two_thirds", |b| {
        b.iter(|| {
            let p_x = black_box(0.5_f32);
            let k = black_box(2.0_f32);
            let r = (k * p_x * 0.5).cbrt().powi(2);
            black_box(r)
        })
    });

    c.bench_function("sdf_powf_one_third", |b| {
        b.iter(|| {
            let p_x = black_box(0.5_f32);
            let k = black_box(2.0_f32);
            let t = (p_x * 0.5 / k).powf(1.0_f32 / 3.0).max(1e-6);
            black_box(t)
        })
    });

    c.bench_function("sdf_cbrt_one_third", |b| {
        b.iter(|| {
            let p_x = black_box(0.5_f32);
            let k = black_box(2.0_f32);
            let t = (p_x * 0.5 / k).cbrt().max(1e-6);
            black_box(t)
        })
    });
}

criterion_group!(benches, bench_powf_optimizations);
criterion_main!(benches);
