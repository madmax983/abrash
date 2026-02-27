use abrash::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_load_obj_does_not_panic(s in "\\PC*") {
        // We expect load_obj to return Ok or Err, but NEVER panic.
        let _ = load_obj(&s);
    }

    #[test]
    fn test_load_obj_huge_integers(
        idx1 in 1_000_000_000usize..usize::MAX,
        idx2 in 1_000_000_000usize..usize::MAX
    ) {
        let s = format!("v 0 0 0\nf {} {}", idx1, idx2);
        let _ = load_obj(&s);
    }
}
