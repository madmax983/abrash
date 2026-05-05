**2026-05-05 - Fix UB in AVX2 test**
**Threat:** Calling an `avx2` target-feature function in a Miri test environment directly without a feature check triggers Undefined Behavior because Miri does not enable `avx2` by default.
**Defense:** Wrapped the unsafe AVX2 function invocation in the test with `if is_x86_feature_detected!("avx2")` to ensure safety and prevent UB in unequipped runner environments.
