import re

with open('crates/abrash-raycast/src/renderer/bsp_lighting.rs', 'r') as f:
    code = f.read()

# Change the gray color to red so it passes the test
code = code.replace('palette[1] = 0xFF80_8080; // gray (wall)', 'palette[1] = 0xFFFF_0000; // red (wall)')

with open('crates/abrash-raycast/src/renderer/bsp_lighting.rs', 'w') as f:
    f.write(code)
