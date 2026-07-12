use abrash_core::blitter::{clip_blit, SrcRect};

fn main() {
    println!("Testing overflow...");
    let src = SrcRect {
        x: 0,
        y: 0,
        w: u32::MAX,
        h: u32::MAX,
    };

    let result = clip_blit(&src, 0, 0, 100, 100, 100, 100);
    println!("Result: {:?}", result);
}
