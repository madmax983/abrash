use abrash::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_load_obj(s in "\\PC*") {
        // Just call load_obj. It should return Ok or Err, but NOT panic.
        let _ = load_obj(&s);
    }
}
