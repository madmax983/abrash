1. **Explore & Identify**
    - [x] Search for undocumented public modules and functions using a python script analyzing `crates/` and `src/`.
    - [x] Read `crates/abrash-render/src/experimental/*.rs` to identify missing docstrings.
2. **Implement Fixes**
    - [x] Add missing documentation for `tunnel.rs`, `mandelbrot.rs`, `precipitation.rs`, `water_ripple.rs`, `modifiers.rs`, and `kaleidoscope.rs`.
    - [x] Add `#[doc(hidden)]` to `fuzz_load_obj` in `obj_loader_fuzz.rs`.
    - [x] Write summary to `.jules/bard.md` as per "Bard" persona guidelines.
3. **Address Clippy Diagnostics**
    - [x] Fix `clippy::doc_markdown` warnings in `post_process/filters.rs` and `experimental/falling_sand.rs` by wrapping hex codes in backticks.
    - [x] Fix `clippy::unreadable_literal`, `clippy::unnecessary_wraps`, `clippy::field_reassign_with_default`, `clippy::semicolon_if_nothing_returned`, and `clippy::imprecise_flops` across tests and examples to pass `cargo clippy --all-targets --all-features -- -D warnings`.
4. **Verification**
    - [x] Run `cargo doc --no-deps` to ensure successful documentation generation.
    - [x] Run `cargo test` to ensure no functionality is broken by documentation or lint updates.
    - [x] Request code review using `request_code_review`.
5. **Pre Commit & Submit**
    - [x] Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
    - [ ] Submit PR using `submit` tool.
