use abrash::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    // Basic fuzzing with random UTF-8 strings
    #[test]
    fn fuzz_load_obj_random_utf8(s in "\\PC*") {
        let _ = load_obj(&s);
    }

    // Fuzzing with huge integers in indices
    #[test]
    fn fuzz_load_obj_huge_indices(
        idx1 in any::<usize>(),
        idx2 in any::<usize>(),
        idx3 in any::<usize>()
    ) {
        let s = format!("v 0 0 0\nv 0 0 0\nv 0 0 0\nf {} {} {}", idx1, idx2, idx3);
        let _ = load_obj(&s);
    }

    // Fuzzing with huge floats
    #[test]
    fn fuzz_load_obj_huge_floats(
        x in any::<f32>(),
        y in any::<f32>(),
        z in any::<f32>()
    ) {
        let s = format!("v {} {} {}", x, y, z);
        let _ = load_obj(&s);
    }

    // Fuzzing with malformed face definitions
    #[test]
    fn fuzz_load_obj_malformed_faces(s in "f [0-9/ ]*") {
        let full_s = format!("v 0 0 0\n{}", s);
        let _ = load_obj(&full_s);
    }
}
