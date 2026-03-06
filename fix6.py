with open("src/experimental/kuwahara.rs", "r") as f:
    code = f.read()

code = code.replace("    let pixels = fb.as_slice();\n", "")

with open("src/experimental/kuwahara.rs", "w") as f:
    f.write(code)
