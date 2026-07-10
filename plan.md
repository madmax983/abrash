1. **Verify Test and Lint Status:**
   - Run `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-targets --all-features` to ensure all tests pass and there are no lint warnings.

2. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**

3. **Submit Pull Request:**
   - Create a PR titled "⚡ Bolt: Replace SipHash with foldhash in Jelly simulation" with a description detailing the performance improvement by avoiding cryptographic hash overhead for integer keys.
