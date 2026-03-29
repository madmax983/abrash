1. Use `run_in_bash_session` to create `crates/abrash-render/src/experimental/matrix_rain.rs` containing the Matrix Digital Rain post-processing filter logic using a heredoc (`cat << 'EOF' > ...\n[EXACT CODE]\nEOF`).
2. Use `run_in_bash_session` to append `pub mod matrix_rain;` to `crates/abrash-render/src/experimental/mod.rs` using `echo 'pub mod matrix_rain;' >> crates/abrash-render/src/experimental/mod.rs`.
3. Use `run_in_bash_session` to run `cargo check -p abrash-render --features nova` to verify the module compiles successfully.
4. Use `run_in_bash_session` with `cat << 'EOF' >> .jules/nova.md` to append the required journal entry for the "Matrix Rain Filter" including Concept, Fate, and Lesson sections.
5. Use `run_in_bash_session` to create `examples/matrix_rain_demo.rs` using a heredoc (`cat << 'EOF' > ...\n[EXACT CODE]\nEOF`).
6. Use `replace_with_git_merge_diff` to add the `matrix_rain_demo` example block to `Cargo.toml`. The exact diff will append the `[[example]]` configuration below the `cube_3d` example block.
7. Use `run_in_bash_session` to verify the example compiles and runs checks by executing `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, and `cargo fmt --all`.
8. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
