1. **Optimize `mesh.rs` face normals**: Change `compute_face_normals` to use `Vec::with_capacity` instead of `map().collect()`, which removes an intermediate allocation/map overhead.
2. **Optimize `gltf_loader.rs` materials**: Change `extract_materials` to use `Vec::with_capacity` and a `for` loop rather than `map().collect()`.
3. **Optimize `lsystem.rs` `expand` method**: Prevent `.clone()` of `current_tls` string and instead reuse and clear the existing buffer using `std::mem::take()`.
4. **Optimize `animator.rs` struct setup**: Delay cloning the `bind_pose` inside `SkeletonAnimator::new()` to the very end of the function.
5. **Optimize `cpu_renderer.rs` `extract_draw_list` method**: Prevent overlapping range indexing which allocates extra structures when mapping over commands in parallel. Use `start..(start + len)` directly.
6. **Execute Pre-commit**: Complete pre-commit steps to make sure proper testing, verifications, reviews and reflections are done.
7. **Submit**: Create PR with a title matching ⚡ Bolt: [performance improvement]
