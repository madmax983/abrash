1. **Optimize Gltf Loader using `with_capacity()` and `.extend()`**
   - Use `run_in_bash_session` to verify that `crates/abrash-skeletal/src/gltf_loader.rs` contains `.collect::<Vec<_>>()` calls for `positions`, `normals`, `uvs`, `tangents`, `joint_indices`, `weights`.
   - The patch replacing `.collect::<Vec<_>>()` with `.with_capacity()` and `.extend()` for `positions`, `normals`, `uvs`, `tangents`, `joint_indices`, and `weights` in `crates/abrash-skeletal/src/gltf_loader.rs` has already been applied in bash.
   - Based strictly on the verified file contents in `crates/abrash-skeletal/src/gltf_loader.rs`.

2. **Replace `.clone()` heap allocations in `create_mesh()` in cpu renderer tests and examples**
   - Use `run_in_bash_session` with `sed` or `patch` to replace `create_mesh` with `create_mesh_owned` in the test functions inside `crates/abrash-render/src/render_api/cpu_renderer.rs`.
   - Based strictly on the verified file contents in `crates/abrash-render/src/render_api/cpu_renderer.rs`.

3. **Verify changes**
   - Use `run_in_bash_session` to run `cargo test --all-targets --all-features` to verify the changes.

4. **Complete pre commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

5. **Submit PR**
   - Submit the PR highlighting the pre-allocation improvements and reduction of unnecessary clone allocations.
