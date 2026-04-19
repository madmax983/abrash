## CRT Retro Filter
**Concept:** A post-processing effect that applies barrel distortion, vignette, chromatic aberration, and scanlines to simulate classic Cathode Ray Tube displays.
**Fate:** Implemented successfully in `crates/abrash-render/src/experimental/crt.rs` with solid multithreaded performance (~9.3ms for an 800x600 buffer).
**Lesson:** Caching normalized coordinates (`nx_cache`) and hoisting multiplier math out of per-pixel channel processing loops yields excellent performance gains. Preallocating intermediate framebuffers via `thread_local!` successfully prevents per-frame allocation overhead while circumventing in-place overwrite tearing.
