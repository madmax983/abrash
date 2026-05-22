import os

def patch_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    lines = content.split('\n')
    modified = False

    for i in range(len(lines)):
        line = lines[i]

        # Look for the combination of clear() and extend_from_slice()
        if '.clear()' in line:
            var_name = line.strip().split('.clear()')[0]

            # We assume extend_from_slice is on the very next line based on the code we saw
            if i + 1 < len(lines):
                next_line = lines[i + 1]
                if f'{var_name}.extend_from_slice' in next_line:
                    # Extract the argument to extend_from_slice
                    start_idx = next_line.find('extend_from_slice(') + len('extend_from_slice(')
                    end_idx = next_line.rfind(')')
                    slice_arg = next_line[start_idx:end_idx]

                    # Replace the lines with resize() and copy_from_slice()
                    lines[i] = line.replace(f'{var_name}.clear()', f'{var_name}.resize({slice_arg}.len(), 0)')
                    lines[i + 1] = next_line.replace('extend_from_slice', 'copy_from_slice')
                    modified = True

    if modified:
        with open(filepath, 'w') as f:
            f.write('\n'.join(lines))
        print(f"Patched {filepath}")

for root, _, files in os.walk('crates/abrash-render/src/experimental'):
    for file in files:
        if file.endswith('.rs'):
            patch_file(os.path.join(root, file))

# And in core too!
for root, _, files in os.walk('crates/abrash-core/src'):
    for file in files:
        if file.endswith('.rs'):
            patch_file(os.path.join(root, file))
