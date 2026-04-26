import re

with open("benches/duotone_bench.rs", "r") as f:
    code = f.read()

code = code.replace("0xFFFF0000", "0xFFFF_0000")
code = code.replace("0xFF0000FF", "0xFF00_00FF")

with open("benches/duotone_bench.rs", "w") as f:
    f.write(code)

with open("examples/slitscan_demo.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), AppError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")

with open("examples/slitscan_demo.rs", "w") as f:
    f.write(code)
