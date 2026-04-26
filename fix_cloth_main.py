import re

with open("examples/cloth_demo.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), Box<dyn Error>> {", "fn main() {")
code = code.replace("winit_demo::run()", "winit_demo::run();")

with open("examples/cloth_demo.rs", "w") as f:
    f.write(code)
