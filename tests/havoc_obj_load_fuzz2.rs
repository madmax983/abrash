use abrash::obj_loader::load_obj;
use proptest::prelude::*;

proptest! {
    #[test]
    fn fuzz_obj_load_panic2(
        v1 in any::<f32>(), v2 in any::<f32>(), v3 in any::<f32>(),
        i1 in any::<i32>(), i2 in any::<i32>(), i3 in any::<i32>(),
        t1 in any::<i32>(), t2 in any::<i32>(), t3 in any::<i32>(),
        n1 in any::<i32>(), n2 in any::<i32>(), n3 in any::<i32>(),
    ) {
        let obj_source = format!("v {} {} {}\n\n\nf {}/{}/{} {}/{}/{} {}/{}/{}\n", v1, v2, v3, i1, t1, n1, i2, t2, n2, i3, t3, n3);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = load_obj(&obj_source);
        }));
    }
}
