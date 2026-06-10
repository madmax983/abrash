import re

with open('crates/abrash-render/src/experimental/arboretum.rs', 'r') as f:
    content = f.read()

content = content.replace('return Err("L-system exceeded memory limits".to_string());', 'return Err(crate::experimental::error::Error::CapacityExceeded("L-system exceeded memory limits"));')

with open('crates/abrash-render/src/experimental/arboretum.rs', 'w') as f:
    f.write(content)
