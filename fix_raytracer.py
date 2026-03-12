with open("src/experimental/raytracer.rs", "r") as f:
    content = f.read()

old = """            for obj in &scene.objects {
                world_aabbs.push(obj.calculate_world_aabb());
            }"""

new = """            world_aabbs.extend(
                scene.objects
                    .iter()
                    .map(|obj| obj.calculate_world_aabb()),
            );"""

if old in content:
    with open("src/experimental/raytracer.rs", "w") as f:
        f.write(content.replace(old, new))
    print("Fixed.")
else:
    print("Not found.")
