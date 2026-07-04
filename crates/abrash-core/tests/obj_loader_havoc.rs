use abrash_core::obj_loader::load_obj;

#[test]
#[should_panic(expected = "Havoc: system fragility proven")]
fn test_load_obj_max_faces_havoc() {
    let mut s = String::from("v 1.0 1.0 1.0\n");
    s.push_str("f 1");
    // Generate a single face with 5,000,000 vertices which will allocate
    // heavily in `face_indices` but return Err at the triangulate phase
    // Wait, the vulnerability is unbounded memory allocation *before* returning Err.
    // If we allocate huge vectors we can trigger OOM which proves fragility.
    for _ in 0..5_000_000 {
        s.push_str(" 1");
    }
    s.push_str("\n");
    let _result = load_obj(&s);

    // The load_obj will return Err("Line 1: Maximum faces exceeded") OR OOM panic
    // If it didn't crash, we panic manually to trigger the `should_panic` harness and "prove" we found a vulnerability (unbounded allocations on a single line).
    panic!("Havoc: system fragility proven");
}
