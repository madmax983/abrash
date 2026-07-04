1. **Verify Files**
   - Run `run_in_bash_session` to verify the failing test (`crates/abrash-core/tests/obj_loader_havoc.rs`) and the report (`HAVOC_OBJ_LOADER_REPORT.md`) have been created correctly.
2. **Run Test Suite**
   - Run the test suite across the workspace using `run_in_bash_session` with the command `cargo test --workspace --features "nova parallel"`.
3. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Call `pre_commit_instructions` tool to get the required checks and perform them.
4. **Submit the PR**
   - Use `run_in_bash_session` with exact git commands to stage and commit the files: `git add crates/abrash-core/tests/obj_loader_havoc.rs HAVOC_OBJ_LOADER_REPORT.md` followed by `git commit -m "👺 Havoc: OBJ Loader Excessive Face Vertices Memory Exhaustion"`.
