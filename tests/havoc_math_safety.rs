use abrash::math::{project_to_screen_optimized, Vec3};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_project_to_screen_safety(
        x in any::<f32>(),
        y in any::<f32>(),
        z in any::<f32>(),
        w in any::<f32>(),
        width in 1.0f32..10000.0,
        height in 1.0f32..10000.0,
    ) {
        let v = Vec3::new(x, y, z);
        let sp = project_to_screen_optimized(v, w, width, height);

        // Assert coordinates are valid i32 and not "poisoned"
        // (Though in Rust i32 is always valid, we care about the logic not spiraling)
        // Main concern is that they are not causing panic during conversion.
        let _ = sp.x;
        let _ = sp.y;
    }
}
