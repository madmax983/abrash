#![cfg(feature = "nova")]
use abrash_render::experimental::lsystem::LSystem;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100_000))]
    #[test]
    fn lsystem_expand_unicode_does_not_crash(axiom in "\\PC*", rule_in in any::<char>(), rule_out in "\\PC*", iterations in 0..10usize, max_cap in 1..1000usize) {
        let mut lsys = LSystem::new(&axiom);
        lsys.add_rule(rule_in, &rule_out);
        lsys.set_max_capacity(max_cap);
        let _ = lsys.expand(iterations);
    }
}
