with open("src/experimental/kuwahara.rs", "r") as f:
    code = f.read()

code = code.replace(
    "use crate::framebuffer::Framebuffer;\n#[cfg(feature = \"parallel\")]\nuse rayon::prelude::*;",
    "use crate::framebuffer::Framebuffer;\n#[cfg(feature = \"parallel\")]\nuse rayon::prelude::*;\nuse std::cell::RefCell;\n\nthread_local! {\n    static KUWAHARA_BUFFER: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };\n}"
)

with open("src/experimental/kuwahara.rs", "w") as f:
    f.write(code)
