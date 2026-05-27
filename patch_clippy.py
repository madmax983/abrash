import sys

file_path = "crates/abrash-render/src/procedural.rs"
with open(file_path, "r") as f:
    content = f.read()

# We need to move `const LUT_SIZE: usize = 1024;` to the top of `plasma_fast` or before it.
if "const LUT_SIZE: usize = 1024;" in content:
    # Remove it from inside the function
    content = content.replace("const LUT_SIZE: usize = 1024;\n", "")
    content = content.replace("const LUT_SIZE: usize = 1024;", "")

    # Place it before the function
    func_sig = "pub fn plasma_fast(width: u32, height: u32) -> Result<Texture, &'static str> {"
    replacement = "const LUT_SIZE: usize = 1024;\n" + func_sig
    content = content.replace(func_sig, replacement)

    with open(file_path, "w") as f:
        f.write(content)
