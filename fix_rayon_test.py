import re

with open('crates/abrash-render/tests/havoc_tile_ub_test.rs', 'r') as f:
    content = f.read()

content = content.replace('#[cfg(feature = "parallel")]\n', '')
content = '#![cfg(feature = "parallel")]\n' + content

with open('crates/abrash-render/tests/havoc_tile_ub_test.rs', 'w') as f:
    f.write(content)
