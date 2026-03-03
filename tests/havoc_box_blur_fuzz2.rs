use abrash::post_process::blur::box_blur_vertical;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_box_blur_vertical(
        width in 0..100usize,
        height in 0..100usize,
        radius in 0..100u32,
        src_len in 0..1000usize,
        dest_len in 0..1000usize,
        acc_len in 0..1000usize,
    ) {
        let src = vec![0u32; src_len];
        let mut dest = vec![0u32; dest_len];
        let mut acc = vec![0i32; acc_len];

        // This will panic when src_len < width * height
        // To make a failing test that acts as a proof of vulnerability, we don't catch the panic.
        box_blur_vertical(&src, &mut dest, &mut acc, width, height, radius);
    }
}
