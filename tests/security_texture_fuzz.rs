use abrash::rasterizer::Texture;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_texture_fuzz_bilinear_fixed(u in any::<i32>(), v in any::<i32>()) {
        let tex = Texture::new(64, 64).unwrap();
        // Should not panic
        let _ = tex.get_pixel_bilinear_fixed(u, v);
    }
}
