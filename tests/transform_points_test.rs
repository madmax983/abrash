use abrash::math::{Mat4, Vec3};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_transform_points_batch(
        points in proptest::collection::vec(
            (
                -1000.0f32..1000.0, // x
                -1000.0f32..1000.0, // y
                -1000.0f32..1000.0  // z
            ),
            0..100 // size
        )
    ) {
        let vec_points: Vec<Vec3> = points.iter().map(|&(x, y, z)| Vec3::new(x, y, z)).collect();
        let mut output = vec![(Vec3::default(), 0.0); vec_points.len()];

        let m = Mat4::rotation_y(0.5) * Mat4::translation(10.0, 5.0, 2.0);

        m.transform_points(&vec_points, &mut output);

        // Verify against individual transform
        for (i, p) in vec_points.iter().enumerate() {
            let (expected_v, expected_w) = m.transform_point(*p);
            let (actual_v, actual_w) = output[i];

            let epsilon = 0.0001;
            assert!((actual_v.x - expected_v.x).abs() < epsilon, "Index {i}: X mismatch");
            assert!((actual_v.y - expected_v.y).abs() < epsilon, "Index {i}: Y mismatch");
            assert!((actual_v.z - expected_v.z).abs() < epsilon, "Index {i}: Z mismatch");
            assert!((actual_w - expected_w).abs() < epsilon, "Index {i}: W mismatch");
        }
    }
}
