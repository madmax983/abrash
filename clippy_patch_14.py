import re

with open('benches/find_min_max_simd_bench.rs', 'r') as f:
    content = f.read()

content = content.replace('format!("Scalar {}x{}", w, h)', 'format!("Scalar {w}x{h}")')
content = content.replace('format!("SIMD {}x{}", w, h)', 'format!("SIMD {w}x{h}")')

with open('benches/find_min_max_simd_bench.rs', 'w') as f:
    f.write(content)
