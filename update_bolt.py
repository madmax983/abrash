import re

file_path = '.jules/bolt.md'
with open(file_path, 'r') as f:
    content = f.read()

new_learning = '''
**[FMA Optimization in Scalar Fallbacks]**
**Learning:** Distributing scalar floating-point math to enable Fused Multiply-Add/Sub (FMA) instructions (e.g., rewriting `((depth - min_z) * scale) as u32` to `(depth * scale - offset) as u32`, where `offset = min_z * scale`) provides significant performance improvements (e.g., ~17% faster) inside hot per-pixel rendering loops when processing scalar tails or fallbacks, by reducing the number of sequential dependent float instructions.
**Action:** Replace `(val - min) * scale` with `val * scale - offset` in hot scalar rendering loops to enable FMA instructions and reduce execution latency.
'''

with open(file_path, 'a') as f:
    f.write(new_learning)
