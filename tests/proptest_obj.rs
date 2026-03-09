use abrash::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    #[test]
    fn integer_overflow_fuzz(s in "v 10000000000000000000000000 0 0\nf [0-9]{1,30} [0-9]{1,30} [0-9]{1,30}") {
        let _ = load_obj(&s);
    }

    #[test]
    fn string_length_fuzz(v_idx in 0..usize::MAX, vt in 0..usize::MAX, vn in 0..usize::MAX) {
        let s = format!("v 0 0 0\nvt 0 0\nvn 0 0 0\nf {}/{}/{} {}/{}/{} {}/{}/{}", v_idx, vt, vn, v_idx, vt, vn, v_idx, vt, vn);
        let _ = load_obj(&s);
    }
}
