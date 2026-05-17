import os
import glob
import re

for test_file in glob.glob("crates/abrash-render/tests/*.rs"):
    with open(test_file, 'r') as f:
        content = f.read()
    if 'abrash_render::experimental' in content and '#[cfg(feature = "nova")]' not in content:
        content = '#[cfg(feature = "nova")]\n' + content
        with open(test_file, 'w') as f:
            f.write(content)
