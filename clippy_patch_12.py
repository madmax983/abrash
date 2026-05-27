import os
import re

files_to_check = [
    "crates/abrash-render/src/experimental/lsystem.rs",
    "crates/abrash-render/src/experimental/arboretum.rs",
    "crates/abrash-render/src/experimental/jelly.rs",
    "crates/abrash-render/src/experimental/steganography.rs"
]

for file in files_to_check:
    if os.path.exists(file):
        with open(file, 'r') as f:
            content = f.read()

        if "crate::experimental::error::Error" not in content:
            # Need to update imports and returns
            content = content.replace("Result<String, &'static str>", "Result<String, crate::experimental::error::Error>")
            content = content.replace("Result<Mesh, &'static str>", "Result<Mesh, crate::experimental::error::Error>")
            content = content.replace("Result<(), &'static str>", "Result<(), crate::experimental::error::Error>")
            content = content.replace("Result<String, String>", "Result<String, crate::experimental::error::Error>")
            content = content.replace("Result<Mesh, String>", "Result<Mesh, crate::experimental::error::Error>")
            content = content.replace("Result<(), String>", "Result<(), crate::experimental::error::Error>")
            content = content.replace("Result<Self, String>", "Result<Self, crate::experimental::error::Error>")

            # Replace explicit errors
            content = content.replace('Err("L-System expansion exceeded maximum capacity limit")', 'Err(crate::experimental::error::Error::CapacityExceeded)')
            content = content.replace('Err("L-System stack overflow")', 'Err(crate::experimental::error::Error::StackOverflow)')
            content = content.replace('Err("L-System stack underflow")', 'Err(crate::experimental::error::Error::StackUnderflow)')
            content = content.replace('Err("Framebuffer too small to hold the message")', 'Err(crate::experimental::error::Error::CapacityExceeded)')
            content = content.replace('ok_or("Message too large")?', 'ok_or(crate::experimental::error::Error::MessageTooLarge)?')

            content = content.replace('Err("L-system exceeded memory limits".to_string())', 'Err(crate::experimental::error::Error::CapacityExceeded)')
            content = content.replace('Err("L-system exceeded memory limits".to_owned())', 'Err(crate::experimental::error::Error::CapacityExceeded)')
            content = content.replace('Err("L-system exceeded maximum stack depth".to_string())', 'Err(crate::experimental::error::Error::StackOverflow)')

            # Jelly replacements
            content = re.sub(r'Err\(format!\(\s*"SoftBody Error: Mesh vertex count \({}\) mismatch with physics state \(v:{}/f:{}\)",[^)]+\)\)', 'Err(crate::experimental::error::Error::MeshIndexOutOfBounds)', content)

            with open(file, 'w') as f:
                f.write(content)
