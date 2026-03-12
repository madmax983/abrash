with open("src/scene.rs", "r") as f:
    content = f.read()

old = """            // 1. Calculate all World AABBs (could be parallelized)
            for obj in &self.objects {
                world_aabbs.push(obj.local_aabb.transform(&obj.transform));
            }"""

new = """            // 1. Calculate all World AABBs (could be parallelized)
            world_aabbs.extend(
                self.objects
                    .iter()
                    .map(|obj| obj.local_aabb.transform(&obj.transform)),
            );"""

if old in content:
    with open("src/scene.rs", "w") as f:
        f.write(content.replace(old, new))
    print("Fixed.")
else:
    print("Not found.")
