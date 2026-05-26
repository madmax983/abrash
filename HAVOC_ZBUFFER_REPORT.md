👺 Havoc: ZBuffer Bounds Checking Bypass via test_and_set_unchecked

🧨 **The Trigger:**
When calling `test_and_set_unchecked` with out of bounds values `(x, y)` (e.g., `(1000, 1000)` on a `100x100` ZBuffer), the function directly uses `slice::get_unchecked_mut`. Because the caller guarantees bounds according to the safety contract, bypassing this contract leads to a severe memory safety violation.

📉 **The Stack Trace:**
```
thread 'havoc_zbuffer_test_and_set_unchecked' panicked at crates/abrash-core/src/zbuffer.rs:199:38:
unsafe precondition(s) violated: slice::get_unchecked_mut requires that the index is within the slice

This indicates a bug in the program. This Undefined Behavior check is optional, and cannot be relied on for safety.
thread caused non-unwinding panic. aborting.
```
In release mode, this results in `(signal: 11, SIGSEGV: invalid memory reference)`.

🧪 **Reproduction:**
Run `cargo test -p abrash-core --test havoc_zbuffer_proptest --release`.

😈 **Comment:**
"You assumed the caller would always guarantee bounds before calling `unchecked`. You were wrong. A simple miscalculation in rasterization math will now bring down the entire application with a SIGSEGV instead of a safe clipping. Stay vigilant."
