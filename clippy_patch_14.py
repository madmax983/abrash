import os

files_to_check = [
    "crates/abrash-render/src/experimental/lsystem.rs",
    "crates/abrash-render/src/experimental/arboretum.rs",
    "crates/abrash-render/src/experimental/jelly.rs",
]

for file in files_to_check:
    if os.path.exists(file):
        with open(file, 'r') as f:
            content = f.read()

        content = content.replace('.map_err(|e| e.to_string())', '.map_err(|_| crate::experimental::error::Error::Utf8Error)')
        content = content.replace('.map_err(|_| "L-System utf8 decoding error")', '.map_err(|_| crate::experimental::error::Error::Utf8Error)')

        # In jelly.rs, fix the 'Other' error constructions
        if "jelly.rs" in file:
            content = content.replace('return Err(format!(', 'return Err(crate::experimental::error::Error::Other(format!(')
            content = content.replace('index_a, index_b\n            ));', 'index_a, index_b\n            )));')
            content = content.replace('v_idx, vertex_count, tri_idx\n                        ));', 'v_idx, vertex_count, tri_idx\n                        )));')

        with open(file, 'w') as f:
            f.write(content)
