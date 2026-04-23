#!/bin/bash
sed -i '239i \
/// Applies a solarize filter to the framebuffer in-place.\
///\
/// Colors with a value above the given threshold will be inverted.\
/// Colors below the threshold remain unchanged.\
///\
/// # Examples\
/// ```\
/// use abrash_core::framebuffer::Framebuffer;\
/// use abrash_render::post_process::filters::apply_solarize;\
///\
/// let mut fb = Framebuffer::new(1, 1).unwrap();\
/// fb.clear(0xFFC0_C0C0); \/\/ Light Gray (192)\
/// apply_solarize(\&mut fb, 127);\
/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);\
/// ```\
pub fn apply_solarize(fb: \&mut Framebuffer, threshold: u8) {\
    let pixels = fb.as_mut_slice();\
    let th = threshold as i32;\
    for pixel in pixels.iter_mut() {\
        let p = *pixel;\
        let a = p \& 0xFF00_0000;\
        let r = (p >> 16) \& 0xFF;\
        let g = (p >> 8) \& 0xFF;\
        let b = p \& 0xFF;\
        \/\/ ⚡ Bolt: Branchless arithmetic for inversion. If value > th, invert (255 - x). Else keep x.\
        \/\/ Uses the sign bit of (th - x) shifted down to create a mask of 1s or 0s.\
        let new_r = r ^ ((((th - r as i32) >> 31) as u32) \& 0xFF);\
        let new_g = g ^ ((((th - g as i32) >> 31) as u32) \& 0xFF);\
        let new_b = b ^ ((((th - b as i32) >> 31) as u32) \& 0xFF);\
        *pixel = a | (new_r << 16) | (new_g << 8) | new_b;\
    }\
}\
' crates/abrash-render/src/post_process/filters.rs
