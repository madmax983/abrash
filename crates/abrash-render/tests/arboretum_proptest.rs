#![allow(missing_docs)]
#![cfg(feature = "nova")]
use abrash_render::experimental::arboretum::LSystem;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100_000))]
    #[test]
    fn arboretum_expand_unicode_does_not_crash(axiom in "\\PC*", rule_in in any::<char>(), rule_out in "\\PC*", iterations in 0..10u32) {
        let mut lsys = LSystem::new(&axiom, 90.0, 1.0, 0.1);
        lsys.add_rule(rule_in, &rule_out);
        let _ = lsys.expand(iterations);
    }
}
