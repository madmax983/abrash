import re

def process_file(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    # Find structs that look like EdgeWalkers
    struct_pattern = re.compile(r'struct (\w+EdgeWalker)\s*\{([^}]+)\}')
    structs = struct_pattern.findall(content)

    for struct_name, struct_body in structs:
        print(f"Found struct: {struct_name}")
        if 'base: EdgeWalker' not in struct_body and 'dx_dy' in struct_body:
            print(f"  Needs refactoring!")

process_file('crates/abrash-render/src/rasterizer/texture.rs')
process_file('crates/abrash-render/src/rasterizer/phong.rs')
process_file('crates/abrash-render/src/rasterizer/reflection.rs')
process_file('crates/abrash-render/src/rasterizer/pbr.rs')
process_file('crates/abrash-render/src/rasterizer/gouraud.rs')
