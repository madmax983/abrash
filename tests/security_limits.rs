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
    #[test]
    fn test_buffer_size_overflow() {
        // width and height are valid individually (<= i32::MAX)
        // but their product exceeds u32::MAX.
        // For example, 65536 * 65536 = 4,294,967,296 (> 4,294,967,295)
        let width = 65536;
        let height = 65536;

        let fb_result = Framebuffer::new(width, height);
        assert!(
            fb_result.is_err(),
            "Framebuffer should reject size > u32::MAX"
        );
        assert_eq!(fb_result.err(), Some("Buffer size overflow"));

        let zb_result = ZBuffer::new(width, height);
        assert!(zb_result.is_err(), "ZBuffer should reject size > u32::MAX");
        assert_eq!(zb_result.err(), Some("Buffer size overflow"));
    }

    #[test]
    #[should_panic(expected = "capacity overflow")]
    fn test_hiz_buffer_capacity_overflow() {
        let _hiz = abrash_core::hiz_buffer::HiZBuffer::new(u32::MAX, u32::MAX);
    }
}
