use std::fmt::Write;
use abrash::obj_loader::load_obj;

#[test]
fn test_obj_deduplication_memory_explosion() {
    let mut obj_source = String::with_capacity(1024 * 1024);
    obj_source.push_str("v 0.0 0.0 0.0\n");

    // Create 9 unique VT coords
    for i in 0..9 {
        let _ = writeln!(obj_source, "vt {} 0.0", i as f32 * 0.1);
    }

    obj_source.push('f');

    // Cycle 1000 times through the 9 variations
    for _ in 0..1000 {
        for i in 1..=9 {
            let _ = write!(obj_source, " 1/{i}");
        }
    }
    obj_source.push('\n');

    let mesh = load_obj(&obj_source).unwrap();

    println!("Mesh vertices: {}", mesh.vertices.len());

    // With HashMap deduplication, this should be exactly 9.
    // With the old buggy implementation, it was 9000.
    assert_eq!(
        mesh.vertices.len(),
        9,
        "Memory explosion or duplicate vertices detected"
    );
}
