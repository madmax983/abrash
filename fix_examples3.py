import os
import re

examples_dir = 'examples'

for filename in os.listdir(examples_dir):
    if not filename.endswith('.rs'):
        continue
    filepath = os.path.join(examples_dir, filename)
    with open(filepath, 'r') as f:
        content = f.read()

    content = re.sub(r'run_windowed\(([^)]+)\)\s*\?;', r'run_windowed(\1);', content)
    content = re.sub(r'run_windowed\((.*)\)\s*\?;', r'run_windowed(\1);', content)

    with open(filepath, 'w') as f:
        f.write(content)
