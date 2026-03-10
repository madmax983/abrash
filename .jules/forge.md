**[Argument Jungles in Rasterization]**
**Learning:** `src/rasterizer/texture.rs` passed up to 15 disparate primitive arguments (coords, UVs, vectors, depths) through its gradient calculation and edge-walking pipelines. This created huge, fragile function signatures.
**Action:** Extract logically grouped parameters into context structs (e.g., `TextureVertex`, `NormalMapVertex`, `TexturedGouraudVertex`). This flattens abstraction bounds, simplifies method arity, and drastically reduces cognitive overhead. Always verify call sites and tests don't break during structurally invasive refactors.
