import re

filepath = 'crates/abrash-core/src/hiz_buffer.rs'
with open(filepath, 'r') as f:
    content = f.read()

search = r"""        let _ = \(width as usize\)
            \.checked_mul\(height as usize\)
            \.and_then\(\|a\| a\.checked_mul\(4\)\)
            \.expect\("Hi-Z dimensions overflow"\);
        let expected_len = \(width as usize\) \* \(height as usize\);"""

replace = """        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .expect("Hi-Z dimensions overflow");

        // WARDEN DEFENSE: Prevent capacity overflow panics
        if expected_len > (isize::MAX as usize) / 4 {
            panic!("Hi-Z dimensions overflow: capacity exceeded");
        }"""

content = re.sub(search, replace, content, count=1)

with open(filepath, 'w') as f:
    f.write(content)
