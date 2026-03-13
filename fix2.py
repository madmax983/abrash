with open("src/rasterizer/tile.rs", "r") as f:
    lines = f.readlines()

new_lines = []

for i, line in enumerate(lines):
    new_lines.append(line)
    if i == 2368 and line.strip() == "}":
        new_lines.append("                        },\n")
        new_lines.append("                    );\n")
        new_lines.append("            }\n")
        new_lines.append("        }\n")

with open("src/rasterizer/tile.rs.tmp", "w") as f:
    f.writelines(new_lines)
