import re

with open("crates/abrash-render/src/post_process/filters.rs", "r") as f:
    code = f.read()

code = code.replace("/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFF_FFFF); // White\n/// Applies a solarize filter to the framebuffer in-place.", "/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFF_FFFF); // White\n/// ```\npub fn apply_invert(fb: &mut Framebuffer) {\n    let pixels = fb.as_mut_slice();\n    for pixel in pixels.iter_mut() {\n        let p = *pixel;\n        let a = p & 0xFF00_0000;\n        let rgb = p & 0x00FF_FFFF;\n        let inv_rgb = rgb ^ 0x00FF_FFFF;\n        *pixel = a | inv_rgb;\n    }\n}\n\n/// Applies a solarize filter to the framebuffer in-place.")
code = code.replace("/// use abrash_render::post_process::filters::apply_solarize`;\n///\n/// ```\n/// use abrash_core::framebuffer::Framebuffer;\n/// use abrash_render::post_process::apply_solarize;\n/// let mut fb = Framebuffer::new(1, 1).unwrap();", "/// let mut fb = Framebuffer::new(1, 1).unwrap();")
code = code.replace("/// apply_solarize(&mut fb, 127);\n/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);\n/// ```\n/// ```", "/// apply_solarize(&mut fb, 127);\n/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);\n/// ```")

with open("crates/abrash-render/src/post_process/filters.rs", "w") as f:
    f.write(code)
