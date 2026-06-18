1. **Refactor `Framebuffer::clear_rect` in `crates/abrash-core/src/framebuffer.rs`:**
   - Update the `else` block (the partial screen clear path).
   - Currently, it calculates offsets manually and uses a `for` loop with `get_unchecked_mut`.
   - Change it to use safe iterator chunking `.chunks_exact_mut(w)` along with `get_unchecked_mut` to elide bounds checks, matching the latest `new_1080p` benchmark that showed a minor performance improvement and significantly improves code readability and reduces explicit offset management.

2. **Refactor `ZBuffer::clear_rect` in `crates/abrash-core/src/zbuffer.rs`:**
   - Similarly update the scalar path of `ZBuffer::clear_rect` to use the same `.chunks_exact_mut(w)` combined with `.get_unchecked_mut(sx..ex)` pattern.
   - This optimizes the scalar Z-buffer clear.

3. **Complete Pre Commit Steps:**
   - Use `pre_commit_instructions` tool to run required checks.

4. **Submit:**
   - Commit and submit the code as "⚡ Bolt: [performance improvement]".
