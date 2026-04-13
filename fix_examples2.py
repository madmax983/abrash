import os
import re

examples_dir = 'examples'

for filename in os.listdir(examples_dir):
    if not filename.endswith('.rs'):
        continue
    filepath = os.path.join(examples_dir, filename)
    with open(filepath, 'r') as f:
        content = f.read()

    changed = False

    if re.search(r'run_windowed\(([^)]+)\?\)', content):
        content = re.sub(r'run_windowed\(([^)]+)\?\)', r'run_windowed(\1.unwrap())', content)
        changed = True

    if 'run_windowed' in content and '?;' in content:
        content = re.sub(r'run_windowed\(([^)]+)\)\s*\?(\s*);', r'run_windowed(\1);', content)
        changed = True

    if re.search(r'run_windowed\(([A-Za-z0-9_:]+::new\([^)]*\))\)', content):
        content = re.sub(r'run_windowed\(([A-Za-z0-9_:]+::new\([^)]*\))\)', r'run_windowed(\1.unwrap())', content)
        changed = True

    if re.search(r'run_windowed\(([A-Za-z0-9_:]+::new\([^)]*\))(\?)?\)', content):
        content = re.sub(r'run_windowed\(([A-Za-z0-9_:]+::new\([^)]*\))(\?)?\)', r'run_windowed(\1.unwrap())', content)
        changed = True

    if re.search(r'run_windowed\(app\)', content):
        content = re.sub(r'(let\s+app\s*=\s*[^;]+)\?;', r'\1.unwrap();', content)
        changed = True

    with open(filepath, 'w') as f:
        f.write(content)
