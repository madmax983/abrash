use abrash::texture::Texture;
use proptest::prelude::*;

#[test]
fn havoc_large_texture_panic_repro() {
    // 2^31 + 1. Fits in u32, but as i32 is negative.
    // This creates a texture that thinks it's valid but crashes on access.
    // AFTER FIX: This should be rejected by Texture::new.
    let width = 0x80000001;
    let height = 1;
    let res = Texture::new(width, height);

    // It should be an error now
    if let Ok(_) = res {
        panic!("Texture::new should have rejected width > i32::MAX");
    }
}

proptest! {
    #[test]
    fn havoc_texture_dimensions_rejected(w in (i32::MAX as u32 + 1)..(i32::MAX as u32 + 1000)) {
        let h = 1;
        let res = Texture::new(w, h);
        // All these must be rejected
        prop_assert!(res.is_err(), "Width {} should be rejected", w);
    }
}
