with open('crates/abrash-render/src/procedural.rs', 'r') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if line.startswith('pub fn plasma'):
        lines.insert(i, "///\n/// # Errors\n/// Returns an error if dimensions are 0 or allocation fails.\n")
        break

with open('crates/abrash-render/src/procedural.rs', 'w') as f:
    f.writelines(lines)
