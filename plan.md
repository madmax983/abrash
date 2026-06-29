1. **Verify Changes**:
   - `cargo test -p abrash-render --features "nova parallel"`

2. **Pre Commit Steps**:
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

3. **Submit**:
   - Use `run_in_bash_session` to push branch and submit PR using: `gh pr create --title "⚡ Bolt: Replace slow modulo with conditional wrapping in Conway hot loop" --body "💡 What: Replaced \`.rem_euclid()\` with conditional addition/subtraction in Conway's Game of Life simulation loop.
🎯 Why: Floating point and integer modulo operators in hot per-pixel loops are very slow due to division instructions.
📊 Impact: Expected to reduce simulation time significantly by eliminating modulo operations.
🔭 Measurement: Verify with tests and benchmarks."`
