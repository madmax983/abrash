use std::fs;

fn main() {
    let mut content = fs::read_to_string("tests/havoc_tile_renderer_fuzz_indices2_proptest.rs").unwrap();
    // Revert what we might have done previously and apply it cleanly
    content = content.replace("#[test]\n    #[should_panic]", "#[test]");
    content = content.replace("#[test]", "#[test]\n    #[should_panic]");
    fs::write("tests/havoc_tile_renderer_fuzz_indices2_proptest.rs", content).unwrap();
}
