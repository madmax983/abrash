#[cfg(feature = "gltf")]
use abrash_skeletal::gltf_loader::{self, load_gltf_scene};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

pub fn gltf_benchmark(c: &mut Criterion) {
    c.bench_function("gltf placeholder", |b| b.iter(|| black_box(0)));
}

criterion_group!(benches, gltf_benchmark);
criterion_main!(benches);
