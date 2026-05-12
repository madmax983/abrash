1. **Remove unused `CoordMode` Enum**:
   - Found in `crates/abrash-gpu-render/src/blitter.rs`. It has two variants (`Pixel`, `Normalized`) and is marked with `#[allow(dead_code)]`. It appears entirely unused ("Zombie Code"). I will delete it entirely.
   - Run `cargo clippy` and `cargo test` to ensure it's successfully removed.
2. **Execute pre-commit instructions**: Follow instructions.
3. **Submit**: Submit the changes.
