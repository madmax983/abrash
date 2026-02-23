use abrash::texture::Texture;

#[test]
fn test_texture_overflow() {
    // This creates a texture that requires 16GB of RAM (4B * 65536 * 65536).
    // It should now return an Error due to overflow check.
    let result = Texture::new(65536, 65536);
    assert!(
        result.is_err(),
        "Texture::new should fail for huge dimensions"
    );
    assert_eq!(
        result.err(),
        Some("Texture size overflow (max i32::MAX pixels)")
    );
}
