use abrash_core::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100_000))]
    #[test]
    fn does_not_crash(s in ".*") {
        let _ = load_obj(&s);
    }
}
