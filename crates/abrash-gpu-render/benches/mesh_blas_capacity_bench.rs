use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_no_reserve(c: &mut Criterion) {
    let meshes: Vec<Option<()>> = vec![Some(()); 50_000];
    c.bench_function("mesh_blas_no_reserve", |b| {
        b.iter(|| {
            let mut mesh_blas: Vec<Option<()>> = Vec::new();
            while mesh_blas.len() < meshes.len() {
                mesh_blas.push(None);
            }
            mesh_blas.push(Some(()));
            black_box(mesh_blas);
        });
    });
}

fn bench_with_reserve(c: &mut Criterion) {
    let meshes: Vec<Option<()>> = vec![Some(()); 50_000];
    c.bench_function("mesh_blas_with_reserve", |b| {
        b.iter(|| {
            let mut mesh_blas: Vec<Option<()>> = Vec::new();
            mesh_blas.reserve_exact(meshes.len().saturating_sub(mesh_blas.len()));
            while mesh_blas.len() < meshes.len() {
                mesh_blas.push(None);
            }
            mesh_blas.push(Some(()));
            black_box(mesh_blas);
        });
    });
}

criterion_group!(benches, bench_no_reserve, bench_with_reserve);
criterion_main!(benches);
