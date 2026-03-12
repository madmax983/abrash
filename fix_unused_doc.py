with open("src/experimental/lsystem.rs", "r") as f:
    content = f.read()

content = content.replace("/// Bolt Performance", "// Bolt Performance")
content = content.replace("/// - Converts commands", "// - Converts commands")
content = content.replace("/// - Pre-calculates the", "// - Pre-calculates the")
content = content.replace("/// - Uses `Mesh::with_capacity`", "// - Uses `Mesh::with_capacity`")
content = content.replace("///   reallocations inside", "//   reallocations inside")

with open("src/experimental/lsystem.rs", "w") as f:
    f.write(content)
