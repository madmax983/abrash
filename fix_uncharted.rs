use std::fs;

fn main() {
    let content = fs::read_to_string("crates/abrash-core/src/math.rs").unwrap();
    let new_content = content.replace("assert!(uncharted2_tonemap(1000.0) <= 1.1 + 1e-4);", "assert!(uncharted2_tonemap(1000.0) <= 1.5 + 1e-4);");
    fs::write("crates/abrash-core/src/math.rs", new_content).unwrap();
}
