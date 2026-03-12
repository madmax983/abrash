import re

files_to_fix = [
    "src/experimental/directional_blur.rs",
    "src/experimental/edge_glow.rs",
    "src/experimental/pixel_sort.rs"
]

for filepath in files_to_fix:
    with open(filepath, 'r') as f:
        content = f.read()

    # We will just remove the long block comment completely to fix the long line thing since it repeats anyway
    content = re.sub(r'/// Replaced \`.chunks_mut\(width\)\` with \`.chunks_exact_mut\(width\)\` to eliminate\n/// remainder chunk handling and bounds checking, enabling better vectorization\n/// and measurable performance improvements\.', '', content)
    content = re.sub(r'///\n\n\n', '///\n', content)

    with open(filepath, 'w') as f:
        f.write(content)
