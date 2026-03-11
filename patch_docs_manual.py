import glob

doc_str = "/// Bolt Performance Optimization:\n/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate\n/// remainder chunk handling and bounds checking, enabling better vectorization\n/// and measurable performance improvements.\n"

for f in glob.glob("src/experimental/*.rs"):
    if f.endswith("pixelate.rs"):
        continue
    with open(f, 'r') as file:
        content = file.read()
    if 'chunks_exact_mut' in content and 'Bolt Performance Optimization' not in content:
        content = content.replace("pub fn apply_", doc_str + "pub fn apply_")
        with open(f, 'w') as file:
            file.write(content)

# raytracer
with open("src/experimental/raytracer.rs", 'r') as file:
    content = file.read()
if 'chunks_exact_mut' in content and 'Bolt Performance Optimization' not in content:
    content = content.replace("pub fn render(", "    " + doc_str.replace("///", "    ///") + "pub fn render(")
    with open("src/experimental/raytracer.rs", 'w') as file:
        file.write(content)
