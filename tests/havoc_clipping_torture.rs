use abrash::clipping::{clip_triangle_to_frustum, ClippedTriangles};
use abrash::math::Vec3;
use proptest::prelude::*;

// A vertex type for testing
#[derive(Debug, Clone, Copy, PartialEq)]
struct TestVertex {
    pos: Vec3,
    w: f32,
}

// Implement Lerp for TestVertex
use abrash::clipping::Lerp;
impl Lerp for TestVertex {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            pos: self.pos.lerp(other.pos, t),
            w: self.w + (other.w - self.w) * t,
        }
    }
}

// Helper to extract pos and w
fn get_pos(v: &TestVertex) -> (Vec3, f32) {
    (v.pos, v.w)
}

prop_compose! {
    fn arb_vertex()(x in any::<f32>(), y in any::<f32>(), z in any::<f32>(), w in any::<f32>()) -> TestVertex {
        TestVertex {
            pos: Vec3::new(x, y, z),
            w
        }
    }
}

proptest! {
    // Torture test with completely random floats (NaN, Inf, etc.)
    #[test]
    fn test_clip_triangle_torture(
        v0 in arb_vertex(),
        v1 in arb_vertex(),
        v2 in arb_vertex()
    ) {
        let _ = clip_triangle_to_frustum(v0, v1, v2, get_pos);
    }

    // Test with specific tricky values
    #[test]
    fn test_clip_triangle_tricky_values(
        v0 in arb_vertex(),
        v1 in arb_vertex(),
        v2 in arb_vertex()
    ) {
        // Run it multiple times or just rely on proptest's shrinking
        let _ = clip_triangle_to_frustum(v0, v1, v2, get_pos);
    }
}

#[test]
fn test_clip_triangle_nan_panic() {
    // Explicitly construct a case that might cause trouble
    // NaN position
    let v0 = TestVertex { pos: Vec3::new(f32::NAN, 0.0, 0.0), w: 1.0 };
    let v1 = TestVertex { pos: Vec3::new(0.0, 0.0, 0.0), w: 1.0 };
    let v2 = TestVertex { pos: Vec3::new(1.0, 1.0, 0.0), w: 1.0 };

    // Should not panic
    let _ = clip_triangle_to_frustum(v0, v1, v2, get_pos);
}

#[test]
fn test_clip_triangle_inf_panic() {
    // Infinity position
    let v0 = TestVertex { pos: Vec3::new(f32::INFINITY, 0.0, 0.0), w: 1.0 };
    let v1 = TestVertex { pos: Vec3::new(0.0, 0.0, 0.0), w: 1.0 };
    let v2 = TestVertex { pos: Vec3::new(1.0, 1.0, 0.0), w: 1.0 };

    // Should not panic
    let _ = clip_triangle_to_frustum(v0, v1, v2, get_pos);
}
