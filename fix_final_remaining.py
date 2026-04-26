import re

with open("examples/jelly_demo.rs", "r") as f:
    code = f.read()

code = code.replace("use std::error::Error;", "")

with open("examples/jelly_demo.rs", "w") as f:
    f.write(code)

with open("examples/selective_color_demo.rs", "r") as f:
    code = f.read()

code = code.replace("0xFF111111", "0xFF11_1111")
code = code.replace("fn main() -> Result<(), HostError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")
code = code.replace("fn new() -> Result<Self, HostError> {", "fn new() -> Self {")
code = code.replace("        Ok(Self {\n            presenter: None,\n            fb: Framebuffer::new(WIDTH, HEIGHT).unwrap(),\n            zb: ZBuffer::new(WIDTH, HEIGHT).unwrap(),\n            mesh,\n            rotation_y: 0.0,\n            rotation_x: 0.0,\n            timer,\n            light_dir: Vec3::new(-1.0, 1.0, -1.0).normalize(),\n            filter_config: SelectiveColorConfig {\n                target_hue: 0.0, // Red\n                tolerance: 45.0,\n                desaturation: 1.0,\n            },\n        })", "        Self {\n            presenter: None,\n            fb: Framebuffer::new(WIDTH, HEIGHT).unwrap(),\n            zb: ZBuffer::new(WIDTH, HEIGHT).unwrap(),\n            mesh,\n            rotation_y: 0.0,\n            rotation_x: 0.0,\n            timer,\n            light_dir: Vec3::new(-1.0, 1.0, -1.0).normalize(),\n            filter_config: SelectiveColorConfig {\n                target_hue: 0.0, // Red\n                tolerance: 45.0,\n                desaturation: 1.0,\n            },\n        }")

with open("examples/selective_color_demo.rs", "w") as f:
    f.write(code)

with open("tests/havoc_draw_line_3d.rs", "r") as f:
    code = f.read()

code = code.replace("0xFFFFFFFF", "0xFFFF_FFFF")

with open("tests/havoc_draw_line_3d.rs", "w") as f:
    f.write(code)

with open("examples/sonar_demo.rs", "r") as f:
    code = f.read()

code = code.replace("let mut config = SonarConfig::default();\n        config.time = self.time;\n        config.wave_spacing = 5.0;\n        config.wave_speed = 3.0;\n        config.wave_color = 0xFF_00_FF_AA;", "let config = SonarConfig { time: self.time, wave_spacing: 5.0, wave_speed: 3.0, wave_color: 0xFF_00_FF_AA, ..Default::default() };")

with open("examples/sonar_demo.rs", "w") as f:
    f.write(code)
