with open("src/culling.rs", "r") as f:
    lines = f.readlines()

with open("src/culling.rs", "w") as f:
    in_func = False
    for line in lines:
        if line.startswith("    pub fn cull_spheres(&self, spheres: &[BoundingSphere]) -> Vec<bool> {"):
            in_func = True
            f.write(line)
            f.write("        let mut results = vec![false; spheres.len()];\n")
            f.write("        self.cull_spheres_into(spheres, &mut results);\n")
            f.write("        results\n")
            f.write("    }\n\n")
            f.write("    pub fn cull_spheres_into(&self, spheres: &[BoundingSphere], results: &mut [bool]) {\n")
            f.write("        let len = spheres.len();\n")
            f.write("        assert_eq!(len, results.len());\n")
            continue

        if in_func and line.strip() == "let mut results = Vec::with_capacity(len);":
            continue

        if in_func and "results.push(" in line:
            f.write(line.replace("results.push(", "results[i] = ").replace(");", ";"))
            continue

        if in_func and "results.extend(" in line:
            f.write(line.replace("results.extend(", "results[i..i + 8].copy_from_slice(&").replace(");", ");"))
            continue

        if in_func and "results" in line and line.strip() == "results":
            in_func = False
            continue

        f.write(line)
