**Optimize allocations and math operations in experimental modules**
**Learning:** Pre-allocating vector capacity with `Vec::with_capacity` and replacing division with multiplication by reciprocal in hot inner loops yields massive speedups (e.g., ~54% faster metaballs benchmarking).
**Action:** Actively scan for dynamic vector creation and division in hot loops during optimization.
