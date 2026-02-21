use abrash::texture::Texture;

#[test]
fn test_api_encapsulation() {
    let tex = Texture::new(10, 10).unwrap();

    // Ensure we can access properties via getters
    assert_eq!(tex.width(), 10);
    assert_eq!(tex.height(), 10);
    assert_eq!(tex.pixels().len(), 100);

    // Verify creation invariants (bounds checking)
    assert!(Texture::new(0, 10).is_err());
    assert!(Texture::new(10, 0).is_err());

    // Verify overflow protection
    assert!(Texture::new(u32::MAX, u32::MAX).is_err());
}
