#!/bin/bash
cat << 'MD' >> .jules/bolt.md

**Fused Multiply-Add over Hypot**
**Learning:** Using `(x * x + y * y).sqrt()` triggers the `clippy::imprecise_flops` lint, but replacing it directly with `x.hypot(y)` causes significant performance regressions. The standard library's `hypot` implementation is accurate but much slower than naive squaring. A better approach that is both highly accurate and fast is to use Fused Multiply-Add (FMA): `x.mul_add(x, y * y).sqrt()`.
**Action:** Replaced instances of `(x * x + y * y).sqrt()` with `x.mul_add(x, y * y).sqrt()` globally in core math, procedural, rendering, and raycast logic, ensuring numerical precision while maintaining or improving benchmark performance without needing to suppress standard clippy lints globally.
MD
