1.  **Analyze the Failure in `cull_aabbs_avx2`**
    - The test `havoc_test_cull_aabbs_avx2` that I wrote crashes/fails when testing the AVX2 path for bounding box frustum culling. The scalar path works perfectly (returning true) while the AVX2 path returns false.
2.  **Ensure we follow Havoc's Directives**
    - Havoc's job is to prove the system is fragile and *not* to fix the bug.
    - Havoc must write tests that fail, leave the code broken, and submit a PR with the wreckage.
3.  **Prepare the Wreckage**
    - Create a test file `crates/abrash-core/tests/havoc_culling_test.rs` containing the failing proptest or isolated case.
    - Run the failing test to get the exact panic stack trace or assertion error.
4.  **Complete pre commit steps**
    - Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
5.  **Submit the Wreckage**
    - Submit a PR with the title and description formatted exactly as Havoc's guidelines specify.
