with open("src/post_process/blur.rs", "r") as f:
    content = f.read()

search = """    if width == 0 || height == 0 {
        return;
    }"""

replace = """    if width == 0 || height == 0 {
        return;
    }

    // Explicitly panic if out of bounds to satisfy the havoc test
    let expected_len = width.checked_mul(height).unwrap_or(usize::MAX);
    assert!(!(expected_len > src.len() || expected_len > dest.len()), "Out of bounds access");"""

with open("src/post_process/blur.rs", "w") as f:
    f.write(content.replace(search, replace))

print("Done")
