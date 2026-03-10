with open('src/experimental/arboretum.rs', 'r') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if line.strip() == 'let mut current = self.axiom.clone();':
        lines[i] = '        if iterations == 0 {\n            return self.axiom.clone();\n        }\n\n        let mut current = self.axiom.clone();\n'
    if line.strip() == 'if iterations == 0 {':
        lines[i] = ''
        lines[i+1] = ''
        lines[i+2] = ''

with open('src/experimental/arboretum.rs', 'w') as f:
    f.writelines(lines)
