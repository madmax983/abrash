#[cfg(test)]
mod tests {
    use abrash::framebuffer::Framebuffer;
    use abrash::zbuffer::ZBuffer;

    #[test]
    fn test_framebuffer_rejects_huge_dimensions() {
        // Attempt to create a framebuffer with width > i32::MAX
        // height = 1 so total size < u32::MAX (allocation would succeed if not for the new check)
        let width = u32::MAX;
        let height = 1;

        let fb_result = Framebuffer::new(width, height);
        assert!(
            fb_result.is_err(),
            "Framebuffer should reject width > i32::MAX"
        );
        assert_eq!(
            fb_result.err(),
            Some("Buffer dimensions too large (max i32::MAX)")
        );

        let height_huge = u32::MAX;
        let width_small = 1;
        let fb_result_h = Framebuffer::new(width_small, height_huge);
        assert!(
            fb_result_h.is_err(),
            "Framebuffer should reject height > i32::MAX"
        );
    }

    #[test]
    fn test_zbuffer_rejects_huge_dimensions() {
        let width = u32::MAX;
        let height = 1;

        let zb_result = ZBuffer::new(width, height);
        assert!(zb_result.is_err(), "ZBuffer should reject width > i32::MAX");
        assert_eq!(
            zb_result.err(),
            Some("Buffer dimensions too large (max i32::MAX)")
        );
    }
}
