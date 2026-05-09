# Plan

1. Create Synthwave Grid Filter (`crates/abrash-render/src/experimental/synthwave_grid.rs`)
   - It will map the lower half of the screen (below the horizon) to a 3D perspective grid moving towards the camera.
   - It will be parameterizable (grid color, sky color, horizon position, speed, grid size).
2. Add Synthwave Grid to `crates/abrash-render/src/experimental/mod.rs`
   - Expose the new module.
3. Add Synthwave Grid Demo (`examples/synthwave_grid_demo.rs`)
   - A visual example that renders the grid over time.
4. Add Synthwave Grid Demo to `src/main.rs`
   - Add it to the `DEMOS` list under `Simulation` category.
   - Update `is_nova_example` if needed.
5. Update `Cargo.toml`
   - Add `[[example]]` section for `synthwave_grid_demo` requiring `["nova"]`.
6. Add Synthwave Grid entry to `.jules/nova.md`
   - Document the new feature.
7. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
