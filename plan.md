1. **Explore undocumented pub structs and fns in `crates/abrash-render/src/experimental`**
   - We need to document structs like `VhsConfig`, `LSystem`, `Turtle`, `Steganography`, etc.
   - We need to document functions like `apply_vhs`, `generate_mesh`, `expand`, `apply_kuwahara`, `decode_message`, etc.

2. **Select the target for documentation**
   - The user has identified gaps in the `experimental` module, specifically around `vhs`, `lsystem`, `steganography`, and `kuwahara`. I will pick one to focus on, as per the Bard persona ("Pick the area where confusion is highest" or "write the guide"). The `lsystem` module seems to have a lot of undocumented or poorly documented items, particularly around the `expand` and `generate_mesh` methods, as well as the `LSystem` and `Turtle` structs themselves. Let's document `lsystem.rs`.

3. **Update `crates/abrash-render/src/experimental/lsystem.rs`**
   - Add missing docstrings.
   - Add doc tests (`/// # Examples`).
   - Add `/// # Errors` where applicable.

4. **Verify the documentation**
   - Run `cargo test`
   - Run `cargo doc --no-deps --open`

5. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Run `cargo clippy`
   - Run `cargo fmt`
