with open('crates/abrash-render/src/heat_vision.rs', 'r') as f:
    code = f.read()

old = """    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    let mut has_content = false;

    for &z in depths {
        if z != f32::INFINITY {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
            has_content = true;
        }
    }"""

new = """    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    let mut has_content = false;

    // Use chunks exact to unroll manually
    let mut chunks = depths.chunks_exact(8);
    for chunk in chunks.by_ref() {
        for &z in chunk {
            if z != f32::INFINITY {
                min_z = min_z.min(z);
                max_z = max_z.max(z);
                has_content = true;
            }
        }
    }
    for &z in chunks.remainder() {
        if z != f32::INFINITY {
            min_z = min_z.min(z);
            max_z = max_z.max(z);
            has_content = true;
        }
    }"""

with open('crates/abrash-render/src/heat_vision.rs', 'w') as f:
    f.write(code.replace(old, new))
