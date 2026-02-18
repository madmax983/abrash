use abrash::texture::Texture;

#[test]
fn test_texture_size_limit_i32_overflow() {
    // 46341 * 46341 = 2,147,488,281 > i32::MAX (2,147,483,647)
    // But fits in u32.
    // This targets the SIMD gather instruction vulnerability where indices are signed i32.
    let width = 46341;
    let height = 46341;

    // Ensure our test constants are correct
    let total_pixels: u64 = (width as u64) * (height as u64);
    assert!(
        total_pixels > i32::MAX as u64,
        "Test case must exceed i32::MAX pixels"
    );
    assert!(
        total_pixels < u32::MAX as u64,
        "Test case must fit in u32 pixels (to pass u32 check)"
    );

    // Attempt to create texture
    // This should fail with an error, NOT panic or OOM.
    let res = Texture::new(width, height);

    assert!(
        res.is_err(),
        "Texture::new should reject total size > i32::MAX"
    );
    let err = res.err().unwrap();
    assert_eq!(
        err, "Texture size overflow (max i32::MAX pixels)",
        "Error message should match"
    );
}
