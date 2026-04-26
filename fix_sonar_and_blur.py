import re

with open("examples/sonar_demo.rs", "r") as f:
    code = f.read()

code = code.replace("let mut config = SonarConfig::default();\n        config.time = self.time;\n        config.wave_spacing = 5.0;\n        config.wave_speed = 3.0;\n        config.wave_color = 0xFF_00_FF_AA;", "let config = SonarConfig { time: self.time, wave_spacing: 5.0, wave_speed: 3.0, wave_color: 0xFF_00_FF_AA, ..Default::default() };")

with open("examples/sonar_demo.rs", "w") as f:
    f.write(code)

with open("examples/directional_blur_demo.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), AppError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")

with open("examples/directional_blur_demo.rs", "w") as f:
    f.write(code)
