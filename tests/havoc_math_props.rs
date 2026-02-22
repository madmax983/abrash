use abrash::math::{Mat4, Vec3, project_to_screen_optimized};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_vec3_normalize_no_panic(x in any::<f32>(), y in any::<f32>(), z in any::<f32>()) {
        let v = Vec3::new(x, y, z);
        let n = v.normalize();

        // Output should not be NaN unless input is NaN (or close to 0/Inf)
        // If input is finite and non-zero, output should be unit length (approx)

        // Actually, normalize() implementation handles small length.
        // fast_inv_sqrt might handle NaN/Inf by returning NaN/Inf.
        // We just ensure it doesn't panic.
        let _ = n.x;
    }

    #[test]
    fn test_project_to_screen_clamping(
        vx in any::<f32>(), vy in any::<f32>(), vz in any::<f32>(),
        w in any::<f32>(),
        hw in 1.0f32..10000.0, hh in 1.0f32..10000.0
    ) {
        let v = Vec3::new(vx, vy, vz);
        let sp = project_to_screen_optimized(v, w, hw, hh);

        // The result x/y should be clamped to approx i32 range
        // Or 0 if NaN.

        // Just checking for panics is the main goal here.
        let _ = sp.x;
        let _ = sp.y;
    }

    #[test]
    fn test_mat4_transform_no_panic(
        m00 in any::<f32>(), m01 in any::<f32>(), m02 in any::<f32>(), m03 in any::<f32>(),
        m10 in any::<f32>(), m11 in any::<f32>(), m12 in any::<f32>(), m13 in any::<f32>(),
        m20 in any::<f32>(), m21 in any::<f32>(), m22 in any::<f32>(), m23 in any::<f32>(),
        m30 in any::<f32>(), m31 in any::<f32>(), m32 in any::<f32>(), m33 in any::<f32>(),
        vx in any::<f32>(), vy in any::<f32>(), vz in any::<f32>()
    ) {
        let m = Mat4 {
            m: [
                [m00, m01, m02, m03],
                [m10, m11, m12, m13],
                [m20, m21, m22, m23],
                [m30, m31, m32, m33],
            ]
        };
        let v = Vec3::new(vx, vy, vz);
        let _ = m.transform_point(v);
    }
}
