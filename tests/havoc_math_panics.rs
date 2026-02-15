use abrash::math::{Mat4, Vec3};
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_math_robustness(
        // Generate extreme float values
        x in any::<f32>(), y in any::<f32>(), z in any::<f32>(),
        m00 in any::<f32>(), m01 in any::<f32>(), m02 in any::<f32>(), m03 in any::<f32>(),
        m10 in any::<f32>(), m11 in any::<f32>(), m12 in any::<f32>(), m13 in any::<f32>(),
        m20 in any::<f32>(), m21 in any::<f32>(), m22 in any::<f32>(), m23 in any::<f32>(),
        m30 in any::<f32>(), m31 in any::<f32>(), m32 in any::<f32>(), m33 in any::<f32>(),
    ) {
        let v = Vec3::new(x, y, z);
        let m = Mat4 {
            m: [
                [m00, m01, m02, m03],
                [m10, m11, m12, m13],
                [m20, m21, m22, m23],
                [m30, m31, m32, m33],
            ]
        };

        // This should not panic, even with NaNs and Infs
        let _ = m.transform_point(v);

        let _ = v.normalize();
        let _ = v.fast_normalize();
    }
}
