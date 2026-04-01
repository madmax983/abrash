**[Sentry: Graceful Wgpu Test Initialization]**
**Learning:** When unit testing wgpu/GPU-dependent logic, direct hardware initialization (e.g., `GpuDevice::new_headless`) can panic on constrained CI runners due to missing backend features (like RayQuery). Using `#[should_panic]` to catch these initialization failures makes the test incapable of properly asserting the actual logic under test, creating a false sense of security.
**Action:** Wrap the initialization in `std::panic::catch_unwind(|| { ... })`. If it yields `Ok(Ok(device))`, proceed with the test assertions; otherwise, return early to gracefully skip the test on unsupported platforms.

**[Sentry: Safe Error Handling over Unwraps]**
**Learning:** Hardcoded `.unwrap()` calls on required rendering targets (like `gbuffer` or `hdr_target`) in pipeline passes cause the entire engine to crash if a pass is called out of order or if the target fails to allocate.
**Action:** Replace `.unwrap()` with `let Some(target) = self.target.as_ref() else { return Err(...) }` (or early returns) and propagate `Result` or `Option` types up the call stack to allow the renderer to skip the pass or recover gracefully.
