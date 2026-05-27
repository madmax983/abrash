import sys

file_path = "crates/abrash-render/src/procedural.rs"
with open(file_path, "r") as f:
    content = f.read()

test_code = """
    #[test]
    fn test_plasma_fast() {
        let tex = plasma_fast(10, 10).unwrap();
        assert_eq!(tex.width(), 10);
        assert_eq!(tex.height(), 10);

        let mut has_non_black = false;
        for &p in tex.pixels.iter() {
            if p != 0xFF_00_00_00 {
                has_non_black = true;
                break;
            }
        }
        assert!(
            has_non_black,
            "Texture should not be entirely black after plasma_fast effect"
        );
    }
}
"""

if "test_plasma_fast" not in content:
    # Find the last brace
    last_brace_idx = content.rfind("}")
    if last_brace_idx != -1:
        content = content[:last_brace_idx] + test_code
        with open(file_path, "w") as f:
            f.write(content)
