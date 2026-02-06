use abrash::rasterizer::Texture;
use proptest::prelude::*;

proptest! {
    // We target the range immediately around the sqrt(2^32) where overflow occurs
    // but the wrapped result is small enough to allocate successfully.
    // 65536^2 = 0 (mod 2^32).
    // 65600^2 approx 2^32 + 8.4e6 -> 32MB buffer. Safe.
    #[test]
    fn havoc_texture_integrity(w in 65536u32..65600u32, h in 65536u32..65600u32) {
        // These dimensions create a logical size > u32::MAX (approx 16GB).
        // But due to overflow, the allocated size will be small (0 to ~32MB).

        let res = std::panic::catch_unwind(|| {
            Texture::new(w, h)
        });

        if let Ok(Ok(tex)) = res {
            let logical_size = u64::from(w) * u64::from(h);
            let actual_size = tex.pixels.len() as u64;

            // This assertion MUST fail if the vulnerability exists
            assert_eq!(logical_size, actual_size,
                "Buffer size mismatch! Input: {w}x{h}. Logical: {logical_size}, Actual: {actual_size}");
        }
    }
}
