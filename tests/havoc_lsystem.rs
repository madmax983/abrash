#![cfg(feature = "nova")]
#![cfg(feature = "nova")]
#![cfg(feature = "nova")]
use abrash_render::experimental::lsystem::LSystem;
use proptest::prelude::*;

proptest! {
    #[test]
    #[ignore = "👹 Havoc: Exposes UTF-8 corruption bug in LSystem expansion"]
    fn test_lsystem_utf8_corruption(axiom in "\\pc*") {
        // We use an arbitrary string that might contain multi-byte characters
        let mut lsys = LSystem::new(&axiom);

        // Add a rule that shouldn't affect anything if the characters aren't present
        lsys.add_rule('A', "B");

        // Expand 1 iteration. If the axiom contains no 'A', the result should be exactly the axiom.
        let result = lsys.expand(1).unwrap();

        // This will fail because `.bytes()` treats multi-byte UTF-8 characters
        // as separate bytes, and falls back to pushing them as individual `char`s,
        // resulting in garbled text rather than matching the rule or retaining the original character.
        // E.g. "🚀" becomes "ð\u{9f}\u{9a}\u{80}"
        prop_assert_eq!(result.replace('B', "A"), axiom);
    }
}
