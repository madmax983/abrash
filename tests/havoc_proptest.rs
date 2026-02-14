
use proptest::prelude::*;
use abrash::obj_loader::load_obj;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn fuzz_load_obj(s in "\\PC*") {
        // Just ensure it doesn't crash
        let _ = load_obj(&s);
    }
}
