use abrash_core::zbuffer::ZBuffer;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50_000))]
    #[test]
    #[ignore = "👺 Havoc: Fuzzing zbuffer bounds handling for integer overflow panics"]
    fn fuzz_zbuffer_clear_rect_bounds(
        sx in any::<i32>(),
        sy in any::<i32>(),
        w in any::<u32>(),
        h in any::<u32>(),
    ) {
        let mut zb = ZBuffer::new(100, 100).unwrap();

        // We catch unwinds because we expect this to panic due to integer overflows
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            zb.clear_rect(sx, sy, w, h);
        }));
    }
}
