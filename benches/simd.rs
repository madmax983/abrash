use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::primitives::fill_triangle_textured;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
use abrash::simd::{dot_sse, fill_triangle_textured_simd, mat4_mul_sse, rcp_ss};

// ============================================================================
// Vec3 Dot Product Benchmarks
// ============================================================================

fn bench_vec3_dot_scalar(c: &mut Criterion) {
    c.bench_function("vec3_dot_scalar", |b| {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let v = Vec3::new(4.0, 5.0, 6.0);
        b.iter(|| black_box(a.dot(v)));
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
fn bench_vec3_dot_sse(c: &mut Criterion) {
    c.bench_function("vec3_dot_sse", |b| {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let v = Vec3::new(4.0, 5.0, 6.0);
        b.iter(|| unsafe { black_box(dot_sse(a, v)) });
    });
}

// ============================================================================
// Mat4 Multiplication Benchmarks
// ============================================================================

fn bench_mat4_mul_scalar(c: &mut Criterion) {
    c.bench_function("mat4_mul_scalar", |b| {
        let a = Mat4::rotation_y(0.5);
        let m = Mat4::translation(1.0, 2.0, 3.0);
        b.iter(|| black_box(a.mul(&m)));
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
fn bench_mat4_mul_sse(c: &mut Criterion) {
    c.bench_function("mat4_mul_sse", |b| {
        let a = Mat4::rotation_y(0.5);
        let m = Mat4::translation(1.0, 2.0, 3.0);
        b.iter(|| unsafe { black_box(mat4_mul_sse(&a, &m)) });
    });
}

fn bench_mat4_mul_chain_scalar(c: &mut Criterion) {
    c.bench_function("mat4_mul_chain_scalar", |b| {
        let proj = Mat4::perspective(1.047, 800.0 / 600.0, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let model = Mat4::rotation_y(0.5);

        b.iter(|| {
            let view_model = view.mul(&model);
            black_box(proj.mul(&view_model))
        });
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
fn bench_mat4_mul_chain_sse(c: &mut Criterion) {
    c.bench_function("mat4_mul_chain_sse", |b| {
        let proj = Mat4::perspective(1.047, 800.0 / 600.0, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let model = Mat4::rotation_y(0.5);

        b.iter(|| unsafe {
            let view_model = mat4_mul_sse(&view, &model);
            black_box(mat4_mul_sse(&proj, &view_model))
        });
    });
}

// ============================================================================
// Reciprocal Benchmarks
// ============================================================================

fn bench_reciprocal_scalar(c: &mut Criterion) {
    c.bench_function("reciprocal_scalar", |b| {
        let x = 4.0f32;
        b.iter(|| black_box(1.0 / x));
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
fn bench_reciprocal_sse(c: &mut Criterion) {
    c.bench_function("reciprocal_sse_rcpss", |b| {
        let x = 4.0f32;
        b.iter(|| unsafe { black_box(rcp_ss(x)) });
    });
}

// ============================================================================
// Textured Triangle Fill Benchmarks
// ============================================================================

fn bench_triangle_textured_scalar(c: &mut Criterion) {
    c.bench_function("triangle_textured_scalar", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);
        let texture = Texture::new(64, 64);

        let v0 = ((Vec3::new(0.0, 2.0, -2.0), 1.0), Vec2::new(0.0, 0.0));
        let v1 = ((Vec3::new(-2.0, -2.0, -2.0), 1.0), Vec2::new(1.0, 0.0));
        let v2 = ((Vec3::new(2.0, -2.0, -2.0), 1.0), Vec2::new(0.5, 1.0));

        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(&texture),
            );
        });
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
fn bench_triangle_textured_simd(c: &mut Criterion) {
    c.bench_function("triangle_textured_simd", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);
        let texture = Texture::new(64, 64);

        let v0 = ((Vec3::new(0.0, 2.0, -2.0), 1.0), Vec2::new(0.0, 0.0));
        let v1 = ((Vec3::new(-2.0, -2.0, -2.0), 1.0), Vec2::new(1.0, 0.0));
        let v2 = ((Vec3::new(2.0, -2.0, -2.0), 1.0), Vec2::new(0.5, 1.0));

        b.iter(|| {
            zb.clear();
            fill_triangle_textured_simd(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(&texture),
            );
        });
    });
}

// Perspective-heavy benchmark (lots of 1/w calculations)
fn bench_triangle_textured_perspective_scalar(c: &mut Criterion) {
    c.bench_function("triangle_textured_perspective_scalar", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);
        let texture = Texture::new(64, 64);

        // Vertices with varying w values (more perspective distortion)
        let v0 = ((Vec3::new(0.0, 2.0, -2.0), 0.5), Vec2::new(0.0, 0.0));
        let v1 = ((Vec3::new(-2.0, -2.0, -2.0), 1.5), Vec2::new(1.0, 0.0));
        let v2 = ((Vec3::new(2.0, -2.0, -2.0), 2.0), Vec2::new(0.5, 1.0));

        b.iter(|| {
            zb.clear();
            fill_triangle_textured(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(&texture),
            );
        });
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
fn bench_triangle_textured_perspective_simd(c: &mut Criterion) {
    c.bench_function("triangle_textured_perspective_simd", |b| {
        let mut fb = Framebuffer::new(800, 600);
        let mut zb = ZBuffer::new(800, 600);
        let texture = Texture::new(64, 64);

        let v0 = ((Vec3::new(0.0, 2.0, -2.0), 0.5), Vec2::new(0.0, 0.0));
        let v1 = ((Vec3::new(-2.0, -2.0, -2.0), 1.5), Vec2::new(1.0, 0.0));
        let v2 = ((Vec3::new(2.0, -2.0, -2.0), 2.0), Vec2::new(0.5, 1.0));

        b.iter(|| {
            zb.clear();
            fill_triangle_textured_simd(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(v2),
                black_box(&texture),
            );
        });
    });
}

// ============================================================================
// Criterion Groups
// ============================================================================

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
criterion_group!(
    benches,
    bench_vec3_dot_scalar,
    bench_vec3_dot_sse,
    bench_mat4_mul_scalar,
    bench_mat4_mul_sse,
    bench_mat4_mul_chain_scalar,
    bench_mat4_mul_chain_sse,
    bench_reciprocal_scalar,
    bench_reciprocal_sse,
    bench_triangle_textured_scalar,
    bench_triangle_textured_simd,
    bench_triangle_textured_perspective_scalar,
    bench_triangle_textured_perspective_simd,
);

#[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
criterion_group!(
    benches,
    bench_vec3_dot_scalar,
    bench_mat4_mul_scalar,
    bench_mat4_mul_chain_scalar,
    bench_reciprocal_scalar,
    bench_triangle_textured_scalar,
    bench_triangle_textured_perspective_scalar,
);

criterion_main!(benches);
