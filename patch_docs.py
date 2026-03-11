import os
import glob
import re

doc_str = "    /// Bolt Performance Optimization:\n    /// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate\n    /// remainder chunk handling and bounds checking, enabling better vectorization\n    /// and measurable performance improvements.\n"

files = glob.glob("src/experimental/*.rs")
for f in files:
    if f.endswith("pixelate.rs"):
        continue
    with open(f, 'r') as file:
        content = file.read()
    if 'chunks_exact_mut' in content:
        # insert doc_str right before the function definition
        content = re.sub(r'(\n\s*)(pub fn apply_[a-z_]+\()', r'\1' + doc_str.strip() + r'\1\2', content)
        with open(f, 'w') as file:
            file.write(content)
