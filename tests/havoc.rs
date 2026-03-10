use abrash::clipping::clip_triangle_to_frustum;
use abrash::math::Mat4;
use abrash::math::Vec3;
use abrash::obj_loader::load_obj;
use abrash::texture::Texture;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))] // More cases for chaos

    #[test]
    fn fuzz_obj_loader(s in "\\PC*") {
        // Feed arbitrary unicode strings to the OBJ loader.
        // It should either return Ok(mesh) or Err(msg), but never panic.
        let _ = load_obj(&s);
    }

    #[test]
    fn fuzz_clipping_extremes(
        v0_x in any::<f32>(), v0_y in any::<f32>(), v0_z in any::<f32>(), v0_w in any::<f32>(),
        v1_x in any::<f32>(), v1_y in any::<f32>(), v1_z in any::<f32>(), v1_w in any::<f32>(),
        v2_x in any::<f32>(), v2_y in any::<f32>(), v2_z in any::<f32>(), v2_w in any::<f32>(),
    ) {
        let v0 = (Vec3::new(v0_x, v0_y, v0_z), v0_w);
        let v1 = (Vec3::new(v1_x, v1_y, v1_z), v1_w);
        let v2 = (Vec3::new(v2_x, v2_y, v2_z), v2_w);

        // This function handles geometric clipping.
        // Even with NaNs, Infinities, or Subnormals, it should not panic.
        // It might return garbage triangles or count=0, but must be safe.
        let _result = clip_triangle_to_frustum(v0, v1, v2, |v| *v, |a, b, t| (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t));
    }

    #[test]
    fn fuzz_texture_sampling(
        u in any::<f32>(),
        v in any::<f32>(),
        lod in any::<f32>(),
    ) {
        // Setup a small texture
        let mut tex = Texture::new(4, 4).unwrap();
        // Fill with some data
        tex.set_pixel(0, 0, 0xFFFF_FFFF);
        tex.generate_mipmaps();

        // Trilinear sampling with arbitrary coordinates and LOD
        // Should handle NaNs, Infs, and extreme values gracefully (clamp or return 0)
        let _pixel = tex.get_pixel_trilinear(u, v, lod);
    }

    #[test]
    fn fuzz_mat4_transform_points(
        m in any::<[f32; 16]>(),
        points in prop::collection::vec(any::<[f32; 3]>(), 0..20)
    ) {
        let mat = Mat4 {
            m: [
                [m[0], m[1], m[2], m[3]],
                [m[4], m[5], m[6], m[7]],
                [m[8], m[9], m[10], m[11]],
                [m[12], m[13], m[14], m[15]],
            ]
        };

        let vec3_points: Vec<Vec3> = points.iter().map(|p| Vec3::new(p[0], p[1], p[2])).collect();
        let mut output = vec![(Vec3::default(), 0.0); vec3_points.len()];

        // This runs the optimized implementation (AVX2 if available)
        mat.transform_points(&vec3_points, &mut output);

        for (i, p) in vec3_points.iter().enumerate() {
            let (res_p, res_w) = output[i];

            // Scalar calc
            let x = mat.m[0][0] * p.x + mat.m[1][0] * p.y + mat.m[2][0] * p.z + mat.m[3][0];
            let y = mat.m[0][1] * p.x + mat.m[1][1] * p.y + mat.m[2][1] * p.z + mat.m[3][1];
            let z = mat.m[0][2] * p.x + mat.m[1][2] * p.y + mat.m[2][2] * p.z + mat.m[3][2];
            let w = mat.m[0][3] * p.x + mat.m[1][3] * p.y + mat.m[2][3] * p.z + mat.m[3][3];

            // Check
            let check = |a: f32, b: f32, name: &str| -> Result<(), TestCaseError> {
                 if a.is_nan() {
                     if !b.is_nan() {
                         return Err(TestCaseError::fail(format!("{name} mismatch: NaN vs {b}")));
                     }
                 } else if a.is_infinite() {
                     if a != b {
                         return Err(TestCaseError::fail(format!("{name} mismatch: Inf vs {b}")));
                     }
                 } else {
                     if b.is_nan() {
                          return Err(TestCaseError::fail(format!("{name} mismatch: {a} vs NaN")));
                     }
                     let diff = (a - b).abs();
                     if diff > 1.0 && diff > a.abs() * 0.1 {
                          return Err(TestCaseError::fail(format!("{name} mismatch: {a} vs {b} (diff {diff})")));
                     }
                 }
                 Ok(())
            };

            check(x, res_p.x, "x")?;
            check(y, res_p.y, "y")?;
            check(z, res_p.z, "z")?;
            check(w, res_w, "w")?;
        }
    }
}
