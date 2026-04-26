import re

with open("examples/topography_demo.rs", "r") as f:
    code = f.read()

code = code.replace("0xFF000000", "0xFF00_0000")
code = code.replace("fn new() -> Result<Self, HostError> {", "fn new() -> Self {")
code = code.replace("        Ok(Self {\n            framebuffer: Framebuffer::new(WIDTH, HEIGHT).unwrap(),\n            src_fb: Framebuffer::new(WIDTH, HEIGHT).unwrap(),\n            presenter: None,\n            time: 0.0,\n        })", "        Self {\n            framebuffer: Framebuffer::new(WIDTH, HEIGHT).unwrap(),\n            src_fb: Framebuffer::new(WIDTH, HEIGHT).unwrap(),\n            presenter: None,\n            time: 0.0,\n        }")

with open("examples/topography_demo.rs", "w") as f:
    f.write(code)
