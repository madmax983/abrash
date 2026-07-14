with open('crates/abrash-render/src/scene.rs', 'r') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if 'draw_list.vertices.reserve(total_vertices);' in line:
        lines.pop(i)
        break

for i, line in enumerate(lines):
    if 'draw_list.batches.reserve(visible_count);' in line:
        lines.pop(i)
        break

for i, line in enumerate(lines):
    if 'Eliminate implicit bounds-checks' in line:
        lines[i] = "            // ⚡ Bolt: Eliminate duplicate `.reserve()` calls before `.reserve_exact()`.\n"
        break

with open('crates/abrash-render/src/scene.rs', 'w') as f:
    f.writelines(lines)
