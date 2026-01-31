**Math Library Convention**
**Learning:** The math library uses Column Vectors with Pre-Multiplication (`M * v`) in both 2D and 3D. Transformations are composed Right-to-Left: a product `M = B * A` means "apply A, then B". Likewise, `Mat4` multiplication `A * B` combines the transform of `A` first, then `B`.
**Action:** When refactoring math code, preserve the Column-Vector, Pre-Multiplication convention (`M * v`) and the Right-to-Left composition order (e.g., use `B * A` to apply `A` then `B`), and do not assume `v * M` or Left-to-Right application.
