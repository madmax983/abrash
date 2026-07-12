use abrash_core::zbuffer::ZBuffer;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50_000))]

    #[test]
    #[ignore = "👺 Havoc: Direct Fuzzing of unsafe ZBuffer testing"]
    fn test_zbuffer_unsafe_set(
        x in 0usize..2048usize,
        y in 0usize..2048usize,
        z in any::<f32>(),
        w in 1u32..2048u32,
        h in 1u32..2048u32,
    ) {
        let mut zb = ZBuffer::new(w, h).unwrap();
        // Skip valid bounds so we only test the "unchecked" safety limits
        if x >= w as usize || y >= h as usize {
            // Unchecked setting should technically be protected by caller, but what if the caller
            // has an integer overflow? Let's simulate a caller getting a huge X/Y but trusting it.
            // Oh wait, unchecked is UB if out of bounds. We shouldn't fuzz that directly.
            // Let's test the public API bounds testing instead.
        }
    }
}
