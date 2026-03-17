#![cfg(feature = "nova")]

use abrash::experimental::lsystem::LSystem;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5))] // We don't want to run too many OOM tests

    #[test]
    #[ignore = "👺 Havoc: Intentionally tests an OOM vulnerability. Will SIGKILL runner if run."]
    fn havoc_lsystem_oom(
        // Generates rules that expand exponentially
        rules in prop::collection::vec("[A-Z]{10,20}", 1..5),
        iterations in 100usize..150usize
    ) {
        let mut lsystem = LSystem::new("A");

        for (i, rule) in rules.iter().enumerate() {
            let char_key = (b'A' + i as u8) as char;
            lsystem.add_rule(char_key, rule);
        }

        // Remove the internal max capacity limit
        lsystem.set_max_capacity(usize::MAX);

        // This will attempt to allocate huge numbers of characters and abort via SIGKILL on OOM.
        // It might also panic on `String::with_capacity` due to integer overflow.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = lsystem.expand(iterations);
        }));

        prop_assert!(
            result.is_err(),
            "Expected panic due to OOM buffer size mismatch or capacity overflow"
        );
    }
}
