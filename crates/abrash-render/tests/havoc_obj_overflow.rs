use abrash_core::obj_loader::load_obj;
use std::fmt::Write as _;

#[test]
fn test_obj_load_dos_overflow_large() {
    let mut obj_source = String::with_capacity(20_000_000);
    obj_source.push_str("v 0.0 0.0 0.0\n");

    let num_verts = 1_000_001;

    for i in 0..num_verts {
        writeln!(obj_source, "vt {} 0.0", i as f32 / num_verts as f32)
            .expect("writing to String cannot fail");
    }

    obj_source.push('f');
    for i in 0..num_verts {
        write!(obj_source, " 1/{}", i + 1).expect("writing to String cannot fail");
    }
    obj_source.push('\n');

    let _mesh = load_obj(&obj_source);
}
