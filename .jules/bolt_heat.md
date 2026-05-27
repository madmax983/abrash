## Heat Vision Math Simplification
**Idea:** Replace scalar subtraction and multiplication `(depth - min_z) * scale` with FMA-friendly logic `depth * scale - offset`.
**Impact:** Small improvement on small resolutions, but minimal change or slight regression on large resolutions, indicating memory bandwidth bounds or pipeline bottlenecks rather than math bounds. Still mathematically sound and eliminates a subtraction in scalar paths.
