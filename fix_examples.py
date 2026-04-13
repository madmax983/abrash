import os
import re

examples_dir = 'examples'

# We look for "fn main() -> Result<(), HostError> {" or "fn main() -> Result<(), abrash::platform::HostError> {"
main_sig_re = re.compile(r'fn\s+main\(\)\s*->\s*Result\s*<\s*\(\)\s*,\s*(?:abrash::platform::)?HostError\s*>\s*\{')

for filename in os.listdir(examples_dir):
    if not filename.endswith('.rs'):
        continue
    filepath = os.path.join(examples_dir, filename)
    with open(filepath, 'r') as f:
        content = f.read()

    if not main_sig_re.search(content):
        continue

    # Update main signature
    content = main_sig_re.sub('fn main() {', content)

    # Let's replace "run_windowed(...)?;" with "run_windowed(...);"
    content = re.sub(r'run_windowed\(([^)]+)\)\s*\?(\s*);', r'run_windowed(\1);', content)

    # Let's replace "run_windowed(...)?\n" with "run_windowed(...)\n" (if it was the last expression)
    content = re.sub(r'run_windowed\(([^)]+)\)\s*\?', r'run_windowed(\1)', content)

    # Let's replace ".map_err(Into::into)" on run_windowed
    content = re.sub(r'run_windowed\(([^)]+)\)\.map_err\([^)]+\)', r'run_windowed(\1)', content)

    # Remove Ok(()) if it's the very last thing before the closing brace of main
    content = re.sub(r'Ok\(\(\)\)\s*\}\s*$', '}', content)

    with open(filepath, 'w') as f:
        f.write(content)

print("Done updating examples.")
