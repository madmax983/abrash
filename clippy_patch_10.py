import re
with open("examples/sonar_demo.rs", "r") as f:
    code = f.read()

new_config = """        let config = SonarConfig {
            time: self.time,
            wave_spacing: 5.0, // closer waves
            wave_speed: 3.0,
            wave_color: 0xFF_00_FF_AA,
            ..Default::default()
        };"""

code = re.sub(r"        let mut config = SonarConfig::default\(\);\n        config\.time = self\.time;\n        config\.wave_spacing = 5\.0; \/\/ closer waves\n        config\.wave_speed = 3\.0;\n        config\.wave_color = 0xFF_00_FF_AA;", new_config, code)

with open("examples/sonar_demo.rs", "w") as f:
    f.write(code)
