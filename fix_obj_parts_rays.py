import re

with open("examples/obj_viewer.rs", "r") as f:
    code = f.read()

code = code.replace("return winit_demo::run(mesh, normals, source_name);", "winit_demo::run(mesh, normals, source_name);\n            return Ok(());")

with open("examples/obj_viewer.rs", "w") as f:
    f.write(code)

with open("examples/particles.rs", "r") as f:
    code = f.read()

code = code.replace("fn main() -> Result<(), AppError> {", "fn main() {")
code = code.replace("    Ok(())\n}", "}")

with open("examples/particles.rs", "w") as f:
    f.write(code)

with open("examples/raytracer_demo.rs", "r") as f:
    code = f.read()

code = code.replace("winit_demo::run()", "winit_demo::run();\n    Ok(())")

with open("examples/raytracer_demo.rs", "w") as f:
    f.write(code)
