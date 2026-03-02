use abrash::math::{Mat4, Vec3};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_transform_points_batch(
        points in proptest::collection::vec(
            (
                -20.0f32..20.0, // x
                -20.0f32..20.0, // y
                -20.0f32..20.0  // z
            ),
            0..100 // size
        ),
        matrix_rows in proptest::array::uniform4(proptest::array::uniform4(-20.0f32..20.0))
    ) {
        let vec_points: Vec<Vec3> = points.iter().map(|&(x, y, z)| Vec3::new(x, y, z)).collect();
        let mut output = vec![(Vec3::default(), 0.0); vec_points.len()];

        let m = Mat4 { m: matrix_rows };

        m.transform_points(&vec_points, &mut output);

        // Verify against individual transform
        for (i, p) in vec_points.iter().enumerate() {
            let (expected_v, expected_w) = m.transform_point(*p);
            let (actual_v, actual_w) = output[i];

            // Increase epsilon slightly because matrix multiplication might accumulate more error with random values
            let epsilon = 0.01;
            assert!((actual_v.x - expected_v.x).abs() < epsilon, "Index {}: X mismatch: {} vs {}", i, actual_v.x, expected_v.x);
            assert!((actual_v.y - expected_v.y).abs() < epsilon, "Index {}: Y mismatch: {} vs {}", i, actual_v.y, expected_v.y);
            assert!((actual_v.z - expected_v.z).abs() < epsilon, "Index {}: Z mismatch: {} vs {}", i, actual_v.z, expected_v.z);
            assert!((actual_w - expected_w).abs() < epsilon, "Index {i}: W mismatch: {actual_w} vs {expected_w}");
        }
    }
}
