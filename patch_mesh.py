with open("src/mesh.rs", "r") as f:
    lines = f.readlines()

with open("src/mesh.rs", "w") as f:
    for line in lines:
        if "        Self {" in line:
            pass # Keep it, but wait for context
        f.write(line)
