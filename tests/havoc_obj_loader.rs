use abrash::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn test_load_obj_arbitrary_string(s in "\\PC*") {
        // Just verify it doesn't panic.
        let _ = load_obj(&s);
    }
}
