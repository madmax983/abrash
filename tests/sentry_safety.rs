#![allow(clippy::unreadable_literal, clippy::float_cmp)]
#[cfg(test)]
mod tests {
    use abrash::framebuffer::Framebuffer;
    use abrash::math::{Mat4, Vec3};
    use abrash::texture::Texture;

    #[test]
    fn test_trilinear_lod_safety() {
        let mut tex = Texture::new(16, 16).unwrap();
        // Fill with some data
        for i in 0..16 * 16 {
            tex.pixels[i] = 0xFFFFFFFF;
        }
        tex.generate_mipmaps();

        // Test normal LOD
        let _ = tex.get_pixel_trilinear(0.5, 0.5, 0.0);
        let _ = tex.get_pixel_trilinear(0.5, 0.5, 1.5);

        // Test Edge Case LODs
        let val_nan = tex.get_pixel_trilinear(0.5, 0.5, f32::NAN);
        // We just care that it doesn't panic
        assert_eq!(val_nan, 0xFFFFFFFF);

        let val_inf = tex.get_pixel_trilinear(0.5, 0.5, f32::INFINITY);
        assert_eq!(val_inf, 0xFFFFFFFF); // Should sample lowest mip (1x1) which is white

        let val_neg_inf = tex.get_pixel_trilinear(0.5, 0.5, f32::NEG_INFINITY);
        assert_eq!(val_neg_inf, 0xFFFFFFFF); // Should sample base level
    }

    #[test]
    fn test_perspective_matrix_safety() {
        // Zero FOV should produce Infinite matrix elements
        let m = Mat4::perspective(0.0, 1.0, 0.1, 100.0);

        // Check if transforming a point with this matrix causes issues
        let v = Vec3::new(0.0, 0.0, -5.0);
        let (_, w) = m.transform_point(v);

        // w might be infinite or NaN
        // We just ensure it returns "something" and doesn't panic inside
        assert!(w.is_infinite() || w.is_nan() || w.is_finite());
    }

    #[test]
    fn test_look_at_coincident_eye_target() {
        let eye = Vec3::new(0.0, 0.0, 0.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);

        let m = Mat4::look_at(eye, target, up);

        let v = Vec3::new(1.0, 2.0, 3.0);
        let (_, w) = m.transform_point(v);

        assert_eq!(w, 1.0);
    }

    #[test]
    fn test_clear_rect_overflow() {
        // This test reproduces a potential panic in clear_rect due to integer overflow
        let mut fb = Framebuffer::new(100, 100).unwrap();

        // x + width = 10 + u32::MAX wraps to 9
        // min(100) -> 9
        // range 10..9 -> Panic!
        fb.clear_rect(10, 10, u32::MAX, 10, 0xFFFFFFFF);
    }

    #[test]
    fn test_framebuffer_new_bounds() {
        // Test giant dimensions (should fail safely)
        assert!(Framebuffer::new(i32::MAX as u32 + 1, 100).is_err());
        assert!(Framebuffer::new(100, i32::MAX as u32 + 1).is_err());

        // Test overflow in total size
        // 100000 * 100000 = 10^10, which fits in u64 but not u32.
        // Framebuffer stores pixels in Vec<u32>. Vec max capacity is isize::MAX (on 64-bit is huge).
        // But Framebuffer::new checks:
        // u64::from(width).checked_mul(u64::from(height)).filter(|&s| u32::try_from(s).is_ok())
        // So it restricts total pixels to u32::MAX (~4 billion).

        let dim = 100_000;
        assert!(Framebuffer::new(dim, dim).is_err());
    }
}
