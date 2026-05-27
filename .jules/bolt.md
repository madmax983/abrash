## [Optimization]
**💡 What:** Optimized `apply_plasma` by hoisting axis-invariant sine calculations out of the inner loop and using fractional math instead of modulo.
**🎯 Why:** Procedural plasma generation was calculating the same X-coordinate sine values repeatedly for every Y-row, and modulo operations on floats are computationally expensive.
**📊 Impact:** Reduced execution time for `plasma_800x600` from ~29.8ms to ~13.0ms (~56% faster execution).
**🔬 Measurement:** Verified via `cargo bench --bench plasma_bench --features nova`.
