import os
import re
from collections import defaultdict

def find_cycles():
    imports = defaultdict(set)
    # very simple heuristic
    for root, dirs, files in os.walk("crates"):
        for file in files:
            if file.endswith(".rs"):
                path = os.path.join(root, file)
                mod_name = file[:-3]
                if mod_name == "mod":
                    mod_name = os.path.basename(root)

                with open(path, "r") as f:
                    content = f.read()
                    for line in content.split("\n"):
                        m = re.match(r"^\s*use\s+(crate::|super::)?([a-zA-Z0-9_]+)", line)
                        if m:
                            target = m.group(2)
                            imports[mod_name].add(target)

    # find cycles
    def dfs(node, visited, stack):
        visited.add(node)
        stack.append(node)
        for neighbor in imports[node]:
            if neighbor in stack:
                cycle = stack[stack.index(neighbor):]
                print("Cycle:", " -> ".join(cycle + [neighbor]))
            elif neighbor not in visited:
                dfs(neighbor, visited, stack)
        stack.pop()

    visited = set()
    for node in list(imports.keys()):
        if node not in visited:
            dfs(node, visited, [])

find_cycles()
