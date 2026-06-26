1. **Optimize `lsystem.rs` indexing inner loops**
   - Replace sequential `.push(b)` in `LSystem::expand` byte collection with array destructuring `extend([b])` to allow LLVM to elide repetitive bounds checks.
   - Hoist the `next_bytes.len() > self.max_capacity` bounds check out of the tight inner loop so that it executes once per expansion iteration rather than evaluating `O(N)` times per byte, avoiding branch evaluation overhead.
2. **Pre commit steps**
   - Ensure proper testing, verification, review, and reflection are done using `pre_commit_instructions`.
3. **Submit the change**
   - Create a PR using the `submit` tool to finalize the optimization.
