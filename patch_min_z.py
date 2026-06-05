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

    // Use unsafe get_unchecked to elide bounds checks, and manual loop over the length
    // which has been shown to give a slight performance edge over standard iterators.
    let len = depths.len();
    let mut i = 0;
    while i < len {
        unsafe {
            let z = *depths.get_unchecked(i);
            if z != f32::INFINITY {
                if z < min_z {
                    min_z = z;
                }
                if z > max_z {
                    max_z = z;
                }
                has_content = true;
            }
        }
        i += 1;
    }"""

with open('crates/abrash-render/src/heat_vision.rs', 'w') as f:
    f.write(code.replace(old, new))
