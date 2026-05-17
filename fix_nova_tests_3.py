import os
import glob
import re

for test_file in glob.glob("crates/abrash-render/tests/*.rs"):
    with open(test_file, 'r') as f:
        content = f.read()
    if '#![cfg(feature = "nova")]' in content and '#[cfg(feature = "nova")]' in content:
        content = content.replace('#[cfg(feature = "nova")]\n', '')
        with open(test_file, 'w') as f:
            f.write(content)
