use abrash::math::Vec3;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_vec3_cross_simd(
        x1 in -1000.0f32..1000.0,
        y1 in -1000.0f32..1000.0,
        z1 in -1000.0f32..1000.0,
        x2 in -1000.0f32..1000.0,
        y2 in -1000.0f32..1000.0,
        z2 in -1000.0f32..1000.0,
    ) {
        let v1 = Vec3::new(x1, y1, z1);
        let v2 = Vec3::new(x2, y2, z2);

        let res = v1.cross(v2);

        let expected_x = v1.y * v2.z - v1.z * v2.y;
        let expected_y = v1.z * v2.x - v1.x * v2.z;
        let expected_z = v1.x * v2.y - v1.y * v2.x;

        let epsilon = 0.0001;
        prop_assert!((res.x - expected_x).abs() < epsilon);
        prop_assert!((res.y - expected_y).abs() < epsilon);
        prop_assert!((res.z - expected_z).abs() < epsilon);
    }

    #[test]
    fn test_vec3_dot_simd(
        x1 in -1000.0f32..1000.0,
        y1 in -1000.0f32..1000.0,
        z1 in -1000.0f32..1000.0,
        x2 in -1000.0f32..1000.0,
        y2 in -1000.0f32..1000.0,
        z2 in -1000.0f32..1000.0,
    ) {
        let v1 = Vec3::new(x1, y1, z1);
        let v2 = Vec3::new(x2, y2, z2);

        let res = v1.dot(v2);

        let expected = v1.x * v2.x + v1.y * v2.y + v1.z * v2.z;

        let epsilon = 0.001; // Dot product sums, so error might accumulate slightly more
        prop_assert!((res - expected).abs() < epsilon, "Res: {}, Expected: {}", res, expected);
    }
}
