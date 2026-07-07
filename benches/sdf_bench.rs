#[cfg(feature = "nova")]
use abrash::experimental::sdf::{SdfObject, SdfPrimitive, SdfScene};
#[cfg(feature = "nova")]
use abrash::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
fn bench_sdf_distance(c: &mut Criterion) {
    let mut scene = SdfScene::new();
    scene.add(SdfObject {
        primitive: SdfPrimitive::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(1.0, 1.0, 1.0),
            radius: 0.5,
        },
        color: 0,
    });
    scene.add(SdfObject {
        primitive: SdfPrimitive::Torus {
            major_radius: 1.0,
            minor_radius: 0.5,
            center: Vec3::new(0.0, 0.0, 0.0),
        },
        color: 0,
    });
    scene.add(SdfObject {
        primitive: SdfPrimitive::Sphere {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 1.0,
        },
        color: 0,
    });

    let mut group = c.benchmark_group("sdf");
    group.bench_function("map_distance", |b| {
        b.iter(|| {
            for x in 0..10 {
                for y in 0..10 {
                    for z in 0..10 {
                        black_box(scene.map(Vec3::new(x as f32, y as f32, z as f32)));
                    }
                }
            }
        });
    });
    group.finish();
}

#[cfg(feature = "nova")]
criterion_group!(benches, bench_sdf_distance);

#[cfg(not(feature = "nova"))]
fn bench_sdf_distance(c: &mut Criterion) {}

#[cfg(not(feature = "nova"))]
criterion_group!(benches, bench_sdf_distance);

criterion_main!(benches);
