use abrash::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    // We want to test many iterations to catch edge cases
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn fuzz_obj_loader_proptest(s in "\\PC*") {
        // \\PC* matches any printable unicode character.
        // This generates arbitrary strings.

        let result = load_obj(&s);
        // We only care that it doesn't panic.
        // Err is expected for garbage.
        let _ = result;
    }

    #[test]
    fn fuzz_obj_loader_structure(
        // Generate a list of tokens to assemble into lines
        tokens in prop::collection::vec("[a-zA-Z0-9/.-]{1,10}", 0..100)
    ) {
        // Join tokens with spaces to form a "valid-ish" structure
        let s = tokens.join(" ");
        let _ = load_obj(&s);
    }

    #[test]
    fn fuzz_obj_loader_huge_integers(
        // Generate strings that look like "f 1/1/1" but with huge numbers
        idx in "[0-9]{20,50}"
    ) {
        let s = format!("f {}/{}/{}", idx, idx, idx);
        let _ = load_obj(&s);
    }
}
