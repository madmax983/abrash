import re

with open("crates/abrash-render/src/post_process/filters.rs", "r") as f:
    code = f.read()

code = code.replace("use abrash_core::framebuffer::Framebuffer`;", "use abrash_core::framebuffer::Framebuffer;")
code = code.replace("use abrash_render::post_process::apply_solarize;\nlet mut fb = Framebuffer::new(1, 1).unwrap();", "fn main() {\nuse abrash_core::framebuffer::Framebuffer;\nuse abrash_render::post_process::apply_solarize;\nlet mut fb = Framebuffer::new(1, 1).unwrap();")
code = code.replace("assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);\n```", "assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);\n}\n```")


with open("crates/abrash-render/src/post_process/filters.rs", "w") as f:
    f.write(code)
