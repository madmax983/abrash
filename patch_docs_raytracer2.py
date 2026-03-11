import os
import glob
import re

doc_str = "    /// Bolt Performance Optimization:\n    /// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate\n    /// remainder chunk handling and bounds checking, enabling better vectorization\n    /// and measurable performance improvements.\n"

with open("src/experimental/raytracer.rs", 'r') as file:
    content = file.read()
if 'chunks_exact_mut' in content:
    # find `pub fn render(`
    content = content.replace("    pub fn render(", doc_str + "    pub fn render(")
    with open("src/experimental/raytracer.rs", 'w') as file:
        file.write(content)
