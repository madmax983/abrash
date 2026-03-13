use abrash::obj_loader::load_obj;
use std::fmt::Write;
use std::time::Instant;

#[test]
fn test_obj_dos_quadratic_blowup() {
    let mut obj_source = String::with_capacity(10_000_000);
    obj_source.push_str("v 0.0 0.0 0.0\n");

    let num_verts = 50_000;

    // Generate many unique UVs
    for i in 0..num_verts {
        let _ = write!(obj_source, "vt {} 0.0\n", i as f32 / num_verts as f32);
    }

    // Generate a single face with many vertices, all using v index 1 but unique vt indices
    obj_source.push('f');
    for i in 0..num_verts {
        let _ = write!(obj_source, " 1/{}", i + 1);
    }
    obj_source.push('\n');

    let start = Instant::now();
    let _mesh = load_obj(&obj_source).unwrap();
    let duration = start.elapsed();

    println!("Loaded {num_verts} vertices in {duration:?}");

    // If it takes more than 1 second, it's likely quadratic
    assert!(
        duration.as_secs_f32() < 1.0,
        "OBJ loading took too long: {duration:?}"
    );
}
