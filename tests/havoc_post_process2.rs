use abrash::post_process::blur::{box_blur_horizontal, box_blur_vertical};

#[test]
fn test_box_blur_horizontal_large_radius() {
    let width = 100usize;
    let height = 100usize;
    let radius = 1000u32; // > width
    let src = vec![0xFFFF_FFFF_u32; width * height];
    let mut dest = vec![0u32; width * height];
    box_blur_horizontal(&src, &mut dest, width, height, radius);
}

#[test]
fn test_box_blur_vertical_large_radius() {
    let width = 100usize;
    let height = 100usize;
    let radius = 1000u32; // > height
    let src = vec![0xFFFF_FFFF_u32; width * height];
    let mut dest = vec![0u32; width * height];
    let mut acc = vec![0i32; width * 3];
    box_blur_vertical(&src, &mut dest, &mut acc, width, height, radius);
}
