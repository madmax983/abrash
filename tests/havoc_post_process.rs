use abrash::post_process::blur::{box_blur_horizontal, box_blur_vertical};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_box_blur_horizontal_fuzz(
        width in 1..2000usize,
        height in 1..2000usize,
        radius in 0..100u32,
    ) {
        let src = vec![0u32; width * height];
        let mut dest = vec![0u32; width * height];
        box_blur_horizontal(&src, &mut dest, width, height, radius);
    }

    #[test]
    fn test_box_blur_vertical_fuzz(
        width in 1..2000usize,
        height in 1..2000usize,
        radius in 0..100u32,
    ) {
        let src = vec![0u32; width * height];
        let mut dest = vec![0u32; width * height];
        let mut acc = vec![0i32; width * 3];
        box_blur_vertical(&src, &mut dest, &mut acc, width, height, radius);
    }
}
