import re

with open("tests/radial_blur_tests.rs", "r") as f:
    code = f.read()
code = code.replace("0xFFFFFFFF", "0xFFFF_FFFF")
with open("tests/radial_blur_tests.rs", "w") as f:
    f.write(code)

with open("benches/harmonograph_bench.rs", "r") as f:
    code = f.read()
code = code.replace("0xFF000000", "0xFF00_0000")
code = code.replace("let mut h = Harmonograph::default();\n    h.iterations = 10000;", "let mut h = Harmonograph { iterations: 10000, ..Default::default() };")
with open("benches/harmonograph_bench.rs", "w") as f:
    f.write(code)

with open("examples/gpu_mvp_cube.rs", "r") as f:
    code = f.read()
code = code.replace("fn main() -> Result<(), DemoError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")
with open("examples/gpu_mvp_cube.rs", "w") as f:
    f.write(code)
