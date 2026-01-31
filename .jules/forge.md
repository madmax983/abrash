**Math Library Convention**
**Learning:** The math library uses Row Vectors with Post-Multiplication (`v * M`). Transformations are applied Left-to-Right (`v * First * Second`). `Mat4` multiplication `A * B` combines `A` then `B`.
**Action:** When refactoring math code, ensure matrix multiplication order preserves the Left-to-Right application sequence.
