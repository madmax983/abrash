1. **Attack - The Harness**: Write a test harness in `tests/havoc_proptest.rs` that explicitly attempts to create extremely large allocations (OOM) via property testing. Add `#[ignore]` since it intentionally panics with `SIGABRT` or hangs the test runner to demonstrate the vulnerability without breaking the standard test suite.
2. **Verify Harness**: Use `cat` to verify that `tests/havoc_proptest.rs` was created correctly.
3. **Execute Harness**: Run `cargo test --test havoc_proptest -- --ignored` to execute the harness.
4. **Execute all tests**: Run `cargo test` to ensure that standard tests still pass and no regressions were introduced.
5. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done**.
6. **Create the PR** with the exact formatting requested by Havoc.
