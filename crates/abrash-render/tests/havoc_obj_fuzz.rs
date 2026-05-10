use abrash_core::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_obj_loader_proptest(s in "\\PC*") {
        let _ = load_obj(&s);
    }
}
