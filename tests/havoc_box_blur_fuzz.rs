use abrash::post_process::box_blur_horizontal;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_box_blur_horizontal(
        width in 0..1000usize,
        height in 0..1000usize,
        radius in 0..1000u32,
        src_len in 0..1000000usize,
        dest_len in 0..1000000usize,
    ) {
        let src = vec![0u32; src_len];
        let mut dest = vec![0u32; dest_len];

        // This should either run cleanly or panic (which we catch)
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            box_blur_horizontal(&src, &mut dest, width, height, radius);
        }));
    }
}
