import re

with open("crates/abrash-render/src/post_process/filters.rs", "r") as f:
    code = f.read()

code = code.replace("/// let mut fb = Framebuffer::new(1, 1).unwrap();\n/// fb.clear(0xFFC0_C0C0); // Light Gray (192)\n/// apply_solarize(&mut fb, 127);\n/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);", "/// use abrash_render::post_process::filters::apply_solarize;\n/// let mut fb = Framebuffer::new(1, 1).unwrap();\n/// fb.clear(0xFFC0_C0C0); // Light Gray (192)\n/// apply_solarize(&mut fb, 127);\n/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);")

with open("crates/abrash-render/src/post_process/filters.rs", "w") as f:
    f.write(code)
