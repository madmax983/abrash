import re

with open("examples/gpu_deferred_showcase.rs", "r") as f:
    content = f.read()

content = content.replace("run_windowed(ShowcaseApp::new().unwrap());", "run_windowed(ShowcaseApp::new());")

with open("examples/gpu_deferred_showcase.rs", "w") as f:
    f.write(content)
