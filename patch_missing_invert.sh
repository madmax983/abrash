#!/bin/bash
sed -i '221i \
/// Inverts the colors of the framebuffer in-place.\
///\
/// This effect negates the RGB channels while preserving the Alpha channel.\
///\
/// # Examples\
///\
/// ```\
/// use abrash_core::framebuffer::Framebuffer;\
/// use abrash_render::post_process::filters::apply_invert;\
///\
/// let mut fb = Framebuffer::new(1, 1).unwrap();\
/// fb.set_pixel(0, 0, 0xFF000000); \/\/ Black\
/// apply_invert(\&mut fb);\
///\
/// \/\/ Alpha is preserved (FF), color is inverted (000000 -> FFFFFF)\
/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFFFFFF); \/\/ White\
/// ```\
pub fn apply_invert(fb: \&mut Framebuffer) {\
    let pixels = fb.as_mut_slice();\
\
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]\
    {\
        if std::is_x86_feature_detected!("avx2") {\
            unsafe { simd::apply_invert_avx2(pixels) };\
            return;\
        }\
    }\
\
    for pixel in pixels.iter_mut() {\
        *pixel ^= 0x00FF_FFFF;\
    }\
}\
' crates/abrash-render/src/post_process/filters.rs
