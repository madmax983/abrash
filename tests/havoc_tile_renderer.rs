#![allow(clippy::unreadable_literal)]
use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::tile_renderer::{ClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use proptest::prelude::*;

// Strategy to generate random Vec3 with potential edge cases
fn vec3_strategy() -> impl Strategy<Value = Vec3> {
    prop_oneof![
        // Normal range
        (any::<f32>(), any::<f32>(), any::<f32>()).prop_map(|(x, y, z)| Vec3::new(x, y, z)),
        // Extreme values
        Just(Vec3::new(f32::NAN, f32::NAN, f32::NAN)),
        Just(Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY)),
        Just(Vec3::new(
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY
        )),
        Just(Vec3::new(0.0, 0.0, 0.0)),
        Just(Vec3::new(1e30, 1e30, 1e30)),
    ]
}

// Strategy for ClipTriangle
fn triangle_strategy() -> impl Strategy<Value = ClipTriangle> {
    (
        (vec3_strategy(), any::<f32>()),
        (vec3_strategy(), any::<f32>()),
        (vec3_strategy(), any::<f32>()),
        any::<u32>(),
    )
}

#[test]
fn havoc_render_batch_crash_test() {
    let width = 256;
    let height = 256;
    let mut renderer = TileRenderer::new(width, height);
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Specific nasty cases
    let nasty_triangles = vec![
        // All NaNs
        (
            (Vec3::new(f32::NAN, f32::NAN, f32::NAN), f32::NAN),
            (Vec3::new(f32::NAN, f32::NAN, f32::NAN), f32::NAN),
            (Vec3::new(f32::NAN, f32::NAN, f32::NAN), f32::NAN),
            0xFFFFFFFF,
        ),
        // Infinities
        (
            (Vec3::new(f32::INFINITY, 0.0, 1.0), 1.0),
            (Vec3::new(0.0, f32::INFINITY, 1.0), 1.0),
            (Vec3::new(0.0, 0.0, 1.0), 1.0),
            0xFFFFFFFF,
        ),
        // W = 0 (Divide by zero potential)
        (
            (Vec3::new(0.0, 0.0, 1.0), 0.0),
            (Vec3::new(1.0, 0.0, 1.0), 0.0),
            (Vec3::new(0.0, 1.0, 1.0), 0.0),
            0xFFFFFFFF,
        ),
        // Huge coordinates
        (
            (Vec3::new(1e30, 0.0, 1.0), 1.0),
            (Vec3::new(0.0, 1e30, 1.0), 1.0),
            (Vec3::new(0.0, 0.0, 1.0), 1.0),
            0xFFFFFFFF,
        ),
        // Fuzzing discovery: overflow
        (
            (Vec3::new(-19461560000000.0, 0.0, 0.0), 311677460000000.0),
            (Vec3::new(f32::NAN, f32::NAN, f32::NAN), -0.0),
            (Vec3::new(0.0, 0.0, 0.0), -9.276624e37),
            0,
        ),
    ];

    // Should not panic
    renderer.render_batch(&mut fb, &mut zb, &nasty_triangles);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn prop_render_random_triangles(triangles in prop::collection::vec(triangle_strategy(), 0..10)) {
        let width = 128; // Small buffer for speed
        let height = 128;
        let mut renderer = TileRenderer::new(width, height);
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        // This should not panic or crash
        renderer.render_batch(&mut fb, &mut zb, &triangles);
    }
}
