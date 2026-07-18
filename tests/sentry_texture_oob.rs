#[cfg(test)]
mod tests {
    use abrash_core::texture::Texture;

    #[test]
    fn test_get_pixel_texel_oob() {
        let mut tex = Texture::new(10, 10).unwrap();
        // Set edges
        tex.set_pixel(0, 0, 1);
        tex.set_pixel(9, 9, 2);
        tex.set_pixel(0, 9, 3);
        tex.set_pixel(9, 0, 4);

        // Test OOB
        assert_eq!(tex.get_pixel_texel(-1, -1), 1);
        assert_eq!(tex.get_pixel_texel(10, 10), 2);
        assert_eq!(tex.get_pixel_texel(-5, 100), 3);
        assert_eq!(tex.get_pixel_texel(50, -50), 4);

        // This is a test for get_pixel as well, since it relies on get_pixel_texel for Point filtering
        assert_eq!(tex.get_pixel(1.5, 1.5), 2);
        assert_eq!(tex.get_pixel(-0.5, -0.5), 1);
    }
}
