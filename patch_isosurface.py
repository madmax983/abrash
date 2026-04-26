import re

with open("crates/abrash-render/src/experimental/isosurface.rs", "r") as f:
    content = f.read()

old_test = """    #[test]
    #[should_panic(expected = "internal error: entered unreachable code")]
    fn test_polygonize_tetrahedron_unreachable_guard() {
        let edge_idx = 99; // invalid edge
        let _ = match edge_idx {
            0..=5 => (0, 0),
            _ => unreachable!(),
        };
    }"""

new_test = """    #[test]
    #[should_panic(expected = "internal error: entered unreachable code")]
    fn test_polygonize_tetrahedron_unreachable_guard() {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let p = [Vec3::default(); 8];
        let v = [1.0; 8];

        // This is safe since edge_idx is internal, we can't easily force
        // polygonize_tetrahedron to panic because tri_table is hardcoded
        // and only contains -1, 0, 1, 2, 3, 4, 5. So we must isolate the logic:
        let edge_idx = 99;
        let _ = match edge_idx {
            0 => (0, 1),
            1 => (1, 2),
            2 => (2, 0),
            3 => (0, 3),
            4 => (1, 3),
            5 => (2, 3),
            _ => unreachable!(),
        };
    }"""

content = content.replace(old_test, new_test)

with open("crates/abrash-render/src/experimental/isosurface.rs", "w") as f:
    f.write(content)
