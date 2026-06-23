1. **Optimize glTF Loader `.collect()` calls**
   - Replace the `.collect::<Vec<_>>()` calls in `crates/abrash-skeletal/src/gltf_loader.rs` that allocate dynamically using the intermediate `Iterator::collect`.
   - Specifically, replace them with `Vec::with_capacity(iter.size_hint().0)` and `vec.extend(iter)` to guarantee pre-allocation for the positions, normals, uvs, tangents, joints, weights, and animations channels arrays.
   - This eliminates intermediate allocation chaining which is commonly not properly optimized by LLVM with complex map operations on glTF iterator types.
2. **Verify changes**
   - Run tests for `abrash-skeletal`.
3. **Complete pre commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
4. **Submit PR**
   - Submit PR with the "⚡ Bolt" persona format detailing the removed intermediate collections.
