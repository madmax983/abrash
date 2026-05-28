import re
import os

def process_file(filepath):
    with open(filepath, "r") as f:
        content = f.read()

    # Find the bench loop and replace the whole thing:
    # let draw_list = scene.extract();
    # renderer.begin_frame();
    # ...
    # to:
    # let mut draw_list = abrash::render_api::draw_list::DrawList::with_capacity(abrash::render_api::frame::FrameCamera::new(scene.camera.view, scene.camera.proj), 128, 1024, 0);
    # ...
    # scene.extract_into(&mut draw_list);

    # Actually, we can just replace `let draw_list = scene.extract();`
    # inside `b.iter(|| { ... })` loops with `scene.extract_into(&mut draw_list);`
    # and define `draw_list` right before `b.iter(...)`

    # Let's use a simpler regex replacement since the structure is very consistent:
    # `c.bench_function("...", |b| {\n        b.iter(|| {\n            let draw_list = scene.extract();`

    content = re.sub(
        r"    c\.bench_function\(([^,]+), \|b\| \{\n        b\.iter\(\|\| \{\n            let draw_list = scene\.extract\(\);",
        r"    let mut draw_list = abrash::render_api::draw_list::DrawList::with_capacity(abrash::render_api::frame::FrameCamera::new(scene.camera.view, scene.camera.proj), scene.objects.len(), scene.objects.len() * 3, 0);\n    c.bench_function(\1, |b| {\n        b.iter(|| {\n            scene.extract_into(&mut draw_list);",
        content
    )

    # for inline bench iter: `b.iter(|| scene.extract());` -> `let mut draw_list = ...; b.iter(|| scene.extract_into(&mut draw_list));`
    content = re.sub(
        r"        b\.iter\(\|\| scene\.extract\(\)\);\n    \}\);",
        r"        let mut draw_list = abrash::render_api::draw_list::DrawList::with_capacity(abrash::render_api::frame::FrameCamera::new(scene.camera.view, scene.camera.proj), scene.objects.len(), scene.objects.len() * 3, 0);\n        b.iter(|| scene.extract_into(&mut draw_list));\n    });",
        content
    )

    with open(filepath, "w") as f:
        f.write(content)

process_file("benches/scene_render.rs")
process_file("benches/object_culling.rs")
