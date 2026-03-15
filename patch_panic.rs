use std::fs;

fn main() {
    let mut content = fs::read_to_string("tests/havoc_tile_renderer_panic.rs").unwrap();
    // Check if it already has should_panic, if not, add it
    if !content.contains("#[should_panic]") {
        content = content.replace("#[test]", "#[test]\n    #[should_panic]");
        fs::write("tests/havoc_tile_renderer_panic.rs", content).unwrap();
    }
}
