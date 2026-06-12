**[Exact Size Iterator Collection]**
**Learning:** Replacing `.collect::<Vec<_>>()` with manual `Vec::with_capacity()` and `.extend()` for `ExactSizeIterator`s (like `gltf` crate accessors) provides zero performance benefit. Rust's standard library `FromIterator` already optimally pre-allocates for iterators with a known size.
**Action:** Do not manually unroll `.collect()` calls into `.extend()` for iterators that already implement `size_hint()` correctly. It introduces technical debt without any actual performance gain.
