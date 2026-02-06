#[cfg(test)]
mod tests {
    use abrash::math::Vec3;
    use abrash::rasterizer::color_to_u32;

    #[test]
    fn test_color_nan_safe() {
        let nan = f32::NAN;
        let color = Vec3::new(nan, nan, nan);
        let result = color_to_u32(color);
        // Assert it defaults to 0 (Black) or a specific safe value, but definitely not crash
        // 0xFF00_0000 is Full Alpha Black
        // We mask out alpha just in case, but alpha should be 255.
        // Wait, color_to_u32 logic: 0xFF00_0000 | (r<<16) ...
        // So result should be 0xFF00_0000.
        assert_eq!(result, 0xFF00_0000);
    }
}
