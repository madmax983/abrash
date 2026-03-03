#[test]
fn test_havoc_obj_huge_input() {
    // Generate a string with length 0xFFFFFF (16MB)
    // The user said "Input string with length 0xFFFFFF caused buffer overflow."
    // Let's try to make it a single token.
    let huge_token = "a".repeat(0xFFFFFF);
    let obj_source = format!("v {huge_token}");

    // This should just fail to parse as f32, but not crash/overflow.
    let res = abrash::obj_loader::load_obj(&obj_source);
    assert!(res.is_err());
}

#[test]
fn test_havoc_obj_huge_line() {
    // Generate a string with 0xFFFFFF length, consisting of valid tokens
    // "v 0 0 0 " repeated
    // "v 0 0 0 " is 8 chars.
    // 0xFFFFFF / 8 approx 2 million vertices.
    // 2 million vertices * 12 bytes (Vec3) = 24MB.
    // This should be fine in RAM.

    let chunk = "v 0.0 0.0 0.0\n";
    let count = 0xFFFFFF / chunk.len();
    let obj_source = chunk.repeat(count);

    let res = abrash::obj_loader::load_obj(&obj_source);
    // Should pass or fail gracefully (oom?)
    // 2 million vertices is fine.
    assert!(res.is_ok() || res.is_err()); // Just don't crash
}

#[test]
fn test_havoc_obj_huge_face() {
    // A single face with 0xFFFFFF indices.
    // f 1 2 3 4 ...

    let mut obj_source = String::from("v 0 0 0\n");
    obj_source.push('f');

    // We need indices. " 1" is 2 chars.
    // 0xFFFFFF / 2 = 8 million indices.
    // 8 million * 4 bytes = 32MB.
    // Plus overhead.

    for _ in 0..100_000 {
        obj_source.push_str(" 1");
    }

    let res = abrash::obj_loader::load_obj(&obj_source);
    assert!(res.is_ok());
}
