use abrash::clipping::clip_triangle_to_frustum;
use abrash::math::Vec3;
use criterion::{Criterion, black_box, criterion_group, criterion_main}; // The new optimized implementation

// Helper structs copied for benchmark legacy implementation

pub struct ClippedTriangles<V> {
    pub tris: [V; 24],
    pub count: usize,
}

fn lerp_vertex(v0: (Vec3, f32), v1: (Vec3, f32), t: f32) -> (Vec3, f32) {
    fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }
    fn lerp_vec3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
        Vec3 {
            x: lerp_f32(a.x, b.x, t),
            y: lerp_f32(a.y, b.y, t),
            z: lerp_f32(a.z, b.z, t),
        }
    }
    (lerp_vec3(v0.0, v1.0, t), lerp_f32(v0.1, v1.1, t))
}

// Legacy implementation (slow version)
fn clip_triangle_to_frustum_legacy(
    v0: (Vec3, f32),
    v1: (Vec3, f32),
    v2: (Vec3, f32),
    get_pos: impl Fn(&(Vec3, f32)) -> (Vec3, f32),
) -> ClippedTriangles<(Vec3, f32)> {
    let (p0, w0) = get_pos(&v0);
    let (p1, w1) = get_pos(&v1);
    let (p2, w2) = get_pos(&v2);

    let inside_mask = |p: Vec3, w: f32| -> u8 {
        let mut mask = 0;
        if p.x >= -w {
            mask |= 1;
        }
        if p.x <= w {
            mask |= 2;
        }
        if p.y >= -w {
            mask |= 4;
        }
        if p.y <= w {
            mask |= 8;
        }
        if p.z >= -w {
            mask |= 16;
        }
        if p.z <= w {
            mask |= 32;
        }
        mask
    };

    let m0 = inside_mask(p0, w0);
    let m1 = inside_mask(p1, w1);
    let m2 = inside_mask(p2, w2);

    let all_in = m0 & m1 & m2;
    if all_in == 0x3F {
        let mut tris = [v0; 24];
        tris[0] = v0;
        tris[1] = v1;
        tris[2] = v2;
        return ClippedTriangles { tris, count: 1 };
    }

    let any_in = m0 | m1 | m2;
    if any_in != 0x3F {
        return ClippedTriangles {
            tris: [v0; 24],
            count: 0,
        };
    }

    let mut buf1 = [v0; 12];
    let mut buf2 = [v0; 12];
    buf1[0] = v0;
    buf1[1] = v1;
    buf1[2] = v2;
    let mut count = 3;

    let planes: [fn(Vec3, f32) -> f32; 6] = [
        |p: Vec3, w: f32| p.x + w,
        |p: Vec3, w: f32| w - p.x,
        |p: Vec3, w: f32| p.y + w,
        |p: Vec3, w: f32| w - p.y,
        |p: Vec3, w: f32| p.z + w,
        |p: Vec3, w: f32| w - p.z,
    ];

    for plane in planes {
        if count == 0 {
            break;
        }
        let mut out_count = 0;
        let prev_idx = count - 1;
        let mut prev_v = buf1[prev_idx];
        let (prev_pos, prev_w) = get_pos(&prev_v);
        let mut prev_d = plane(prev_pos, prev_w);

        for &curr_v in buf1.iter().take(count) {
            let (curr_pos, curr_w) = get_pos(&curr_v);
            let curr_d = plane(curr_pos, curr_w);

            if curr_d >= 0.0 {
                if prev_d < 0.0 {
                    let t = prev_d / (prev_d - curr_d);
                    if out_count < 12 {
                        buf2[out_count] = lerp_vertex(prev_v, curr_v, t);
                        out_count += 1;
                    }
                }
                if out_count < 12 {
                    buf2[out_count] = curr_v;
                    out_count += 1;
                }
            } else if prev_d >= 0.0 {
                let t = prev_d / (prev_d - curr_d);
                if out_count < 12 {
                    buf2[out_count] = lerp_vertex(prev_v, curr_v, t);
                    out_count += 1;
                }
            }
            prev_v = curr_v;
            prev_d = curr_d;
        }
        count = out_count;
        buf1[..count].copy_from_slice(&buf2[..count]);
    }

    let mut result = ClippedTriangles {
        tris: [v0; 24],
        count: 0,
    };
    if count >= 3 {
        let pivot = buf1[0];
        for i in 1..count - 1 {
            if result.count < 8 {
                let idx = result.count * 3;
                result.tris[idx] = pivot;
                result.tris[idx + 1] = buf1[i];
                result.tris[idx + 2] = buf1[i + 1];
                result.count += 1;
            }
        }
    }
    result
}

fn bench_clipping(c: &mut Criterion) {
    let mut group = c.benchmark_group("Clipping Microbenchmark");

    // Case 1: Trivial Accept
    group.bench_function("trivial_accept_legacy", |b| {
        let v0 = (Vec3::new(0.0, 0.0, 0.0), 1.0);
        let v1 = (Vec3::new(0.5, 0.0, 0.0), 1.0);
        let v2 = (Vec3::new(0.0, 0.5, 0.0), 1.0);
        b.iter(|| {
            clip_triangle_to_frustum_legacy(black_box(v0), black_box(v1), black_box(v2), |v| *v)
        });
    });

    group.bench_function("trivial_accept_optimized", |b| {
        let v0 = (Vec3::new(0.0, 0.0, 0.0), 1.0);
        let v1 = (Vec3::new(0.5, 0.0, 0.0), 1.0);
        let v2 = (Vec3::new(0.0, 0.5, 0.0), 1.0);
        b.iter(|| {
            clip_triangle_to_frustum(
                black_box(v0),
                black_box(v1),
                black_box(v2),
                |v| *v,
                |a, b, t| (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t),
            )
        });
    });

    // Case 2: Clipping needed (straddling near plane)
    group.bench_function("clipping_needed_legacy", |b| {
        let v0 = (Vec3::new(0.0, 0.0, 1.0), 1.0);
        let v1 = (Vec3::new(0.0, 2.0, -1.0), -1.0);
        let v2 = (Vec3::new(2.0, 0.0, -1.0), -1.0);
        b.iter(|| {
            clip_triangle_to_frustum_legacy(black_box(v0), black_box(v1), black_box(v2), |v| *v)
        });
    });

    group.bench_function("clipping_needed_optimized", |b| {
        let v0 = (Vec3::new(0.0, 0.0, 1.0), 1.0);
        let v1 = (Vec3::new(0.0, 2.0, -1.0), -1.0);
        let v2 = (Vec3::new(2.0, 0.0, -1.0), -1.0);
        b.iter(|| {
            clip_triangle_to_frustum(
                black_box(v0),
                black_box(v1),
                black_box(v2),
                |v| *v,
                |a, b, t| (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t),
            )
        });
    });

    group.finish();
}

criterion_group!(benches, bench_clipping);
criterion_main!(benches);
