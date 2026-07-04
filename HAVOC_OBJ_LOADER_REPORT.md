👺 Havoc: OBJ Loader Excessive Face Vertices Memory Exhaustion

🧊 **The Trigger:**
An OBJ file containing a single face (`f`) definition with millions of vertices (e.g., `f 1 1 1 1 1 1 ...`). The parser attempts to triangulate the fan, but fails to check if the generated triangle count exceeds `MAX_FACES` in advance. It checks on each triangle generated, but the check uses `self.final_indices.len() >= MAX_FACES`, which only triggers when we hit it. More importantly, prior to checking, the `face_indices` vector gathers millions of indices via `parse_face` and `process_vertex_indices`. This triggers massive allocations within `face_indices`, `final_vertices`, `final_uvs`, `final_normals`, and `deduplicator` bypassing the `MAX_VERTICES` and `MAX_FACES` limits on a per-face level.

📉 **The Stack Trace:**
Allocations exceed available RAM and the process gets killed (OOM).

🧪 **Reproduction:**
```rust
use abrash_core::obj_loader::load_obj;

#[test]
fn test_load_obj_max_faces() {
    let mut s = String::from("v 1.0 1.0 1.0\n");
    s.push_str("f 1");
    for _ in 0..1000000 {
        s.push_str(" 1");
    }
    s.push_str("\n");
    let result = load_obj(&s);
    println!("{:?}", result.err());
}
```

😈 **Comment:**
"You assumed that a face would only have 3 or 4 vertices. You didn't expect a user to create a single face with a million vertices. The limits only check total counts, not the counts within a single line. The `face_indices` vector can grow unbounded until memory exhaustion."
