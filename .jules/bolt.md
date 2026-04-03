**[Reusing Vecs to Elide per-frame allocations]**
**Learning:** `Vec::collect()` inside a per-frame render loop (like `flush_to_view` in `GpuBlitter`) results in a dynamic heap allocation every time it's called. This can be elided by keeping a pre-allocated vector inside the parent structure.
**Action:** Add a `Vec<T>` to the main structure (e.g. `GpuBlitter { instances: Vec<SpriteInstance> }`), and in the hot path use `self.instances.clear(); self.instances.extend(...)` instead of `.collect::<Vec<_>>()`. This prevents the recurring heap allocation overhead while maintaining memory safety.
