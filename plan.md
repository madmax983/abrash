1. **Optimize `encode_message` in `crates/abrash-render/src/experimental/steganography.rs`**:
   - The current implementation of `encode_message` creates a `Vec::with_capacity` and calls `extend_from_slice` twice to collect bytes before iterating over them bit-by-bit.
   - I will replace the `data_to_encode` vector allocation completely by chaining iterators: `len.to_le_bytes().into_iter().chain(bytes.iter().copied())`.
   - I will iterate over this chained iterator, applying the bitwise modification logic directly, eliminating the need to allocate an intermediate `Vec<u8>`.

2. **Add a "Bolt's Journal" entry**:
   - I will create or append to `.jules/bolt.md` documenting this insight on chaining iterators to avoid intermediate heap allocations in serialization/encoding loops.

3. **Verify the changes**:
   - I will run `cargo test -p abrash-render --features nova,parallel --lib` to ensure the changes don't break existing behavior.
   - I will run `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --all`.

4. **Complete pre-commit steps**:
   - I will use the `pre_commit_instructions` tool to execute any required pre-commit tasks before submitting the PR.

5. **Submit the PR**:
   - Create a PR with title `⚡ Bolt: Zero-allocation steganography encode` and a description outlining the "What", "Why", "Impact", and "Measurement".
