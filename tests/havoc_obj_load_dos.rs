use abrash::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_obj_load_panic(
        obj_source in ".*",
    ) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = load_obj(&obj_source);
        }));
    }
}
