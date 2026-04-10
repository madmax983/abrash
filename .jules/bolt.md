## [.zip Iterator for SoftBody collide_sdf]
**What:** Replaced index-based `for i in 0..len` loop in `SoftBody::collide_sdf` with `.iter_mut().zip(...)`.
**Why:** Elides bounds checking and satisfies idiomatic Rust patterns.
**Impact:** Minor but consistent performance win.
**Measurement:** `softbody_bench` run time decreased from ~32.1µs to ~29.4µs (~8% improvement).
