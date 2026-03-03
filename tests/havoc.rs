#![allow(clippy::unreadable_literal)]
use abrash::clipping::clip_triangle_to_frustum;
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
        let _result = clip_triangle_to_frustum(v0, v1, v2, |v| *v);
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
        tex.set_pixel(0, 0, 0xFFFFFFFF);
        tex.generate_mipmaps();

        // Trilinear sampling with arbitrary coordinates and LOD
        // Should handle NaNs, Infs, and extreme values gracefully (clamp or return 0)
        let _pixel = tex.get_pixel_trilinear(u, v, lod);
    }
}
