use abrash::obj_loader;
use criterion::{Criterion, criterion_group, criterion_main};
use std::fmt::Write;

fn generate_sphere_obj(rings: usize, sectors: usize) -> String {
    let mut obj = String::with_capacity(rings * sectors * 100);

    // Vertices
    for r in 0..=rings {
        let v = r as f32 / rings as f32; // 0 to 1
        let theta = v * std::f32::consts::PI;

        for s in 0..=sectors {
            let u = s as f32 / sectors as f32; // 0 to 1
            let phi = u * 2.0 * std::f32::consts::PI;

            let x = theta.sin() * phi.cos();
            let y = theta.cos();
            let z = theta.sin() * phi.sin();

            writeln!(obj, "v {x} {y} {z}").unwrap();
            writeln!(obj, "vt {u} {v}").unwrap();
        }
    }

    // Faces
    for r in 0..rings {
        for s in 0..sectors {
            let first = (r * (sectors + 1)) + s + 1;
            let second = first + sectors + 1;

            writeln!(obj, "f {}/{} {}/{} {}/{} {}/{}",
                first, first,
                first + 1, first + 1,
                second + 1, second + 1,
                second, second
            ).unwrap();
        }
    }

    obj
}

fn bench_obj_loading(c: &mut Criterion) {
    // Generate a reasonably complex mesh (approx 50x50 = 2500 quads = 5000 tris)
    let obj_source = generate_sphere_obj(50, 50);

    let mut group = c.benchmark_group("obj_loading");

    group.bench_function("load_sphere_50x50", |b| {
        b.iter(|| {
            obj_loader::load_obj(&obj_source).unwrap()
        });
    });

    group.finish();
}

criterion_group!(benches, bench_obj_loading);
criterion_main!(benches);
