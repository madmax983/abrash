use abrash::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_obj_load_panic3(
        content in any::<String>(),
    ) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = load_obj(&content);
        }));
    }
}
