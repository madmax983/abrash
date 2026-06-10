import re

with open('crates/abrash-render/src/experimental/jelly.rs', 'r') as f:
    content = f.read()

content = re.sub(
    r'return Err\(format!\(\s*\"SoftBody Error: Missing vertex for spring index \{\}\",\s*idx\s*\)\);',
    r'return Err(crate::experimental::error::Error::InvalidInput("SoftBody Error: Missing vertex for spring index"));',
    content
)

content = re.sub(
    r'return Err\(format!\(\s*\"SoftBody Error: Mesh vertex count \(\{\}\) mismatch with physics state \(v:\{\}/f:\{\}\)\",\s*self\.mesh\.vertices\.len\(\),\s*self\.velocities\.len\(\),\s*self\.forces\.len\(\)\s*\)\);',
    r'return Err(crate::experimental::error::Error::InvalidInput("SoftBody Error: Mesh vertex count mismatch with physics state"));',
    content
)

content = re.sub(
    r'return Err\(format!\(\s*\"Spring indices out of bounds: \{\}, \{\}\",\s*index_a, index_b\s*\)\);',
    r'return Err(crate::experimental::error::Error::MeshIndexOutOfBounds(std::cmp::max(index_a, index_b), max_idx));',
    content
)

content = re.sub(
    r'return Err\(format!\(\s*\"Mesh index \{\} out of bounds \(vertex count: \{\}\) at triangle \{\}\",\s*v_idx, vertex_count, tri_idx\s*\)\);',
    r'return Err(crate::experimental::error::Error::MeshIndexOutOfBounds(v_idx, vertex_count));',
    content
)

with open('crates/abrash-render/src/experimental/jelly.rs', 'w') as f:
    f.write(content)
