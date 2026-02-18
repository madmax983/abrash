use abrash::texture::Texture;

#[test]
fn test_texture_encapsulation_prevents_resize() {
    let mut tex = Texture::new(10, 10).unwrap();

    // We can access pixels mutably via the new API
    {
        let pixels = tex.pixels_mut();
        pixels[0] = 0xFFFFFFFF;
        // pixels.clear(); // This would fail to compile now as `&mut [u32]` has no `clear` or `resize`
    }

    // Invariant holds
    assert_eq!(tex.width() * tex.height(), tex.pixels().len() as u32);
}
