with open('src/experimental/arboretum.rs', 'r') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if line.strip() == '#[derive(Clone, Debug)]':
        lines[i] = '#[derive(Clone, Copy, Debug)]\n'
    if line.strip() == 'stack.push(turtle.clone());':
        lines[i] = '                    stack.push(turtle);\n'

with open('src/experimental/arboretum.rs', 'w') as f:
    f.writelines(lines)
