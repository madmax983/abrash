import re

with open("examples/gpu_mvp_cube.rs", "r") as f:
    content = f.read()

content = content.replace("run_windowed(GpuMvpCubeApp::new().unwrap());", "run_windowed(GpuMvpCubeApp::new());")

with open("examples/gpu_mvp_cube.rs", "w") as f:
    f.write(content)
