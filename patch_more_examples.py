import glob
import re

# Let's fix a few more common examples that might panic
examples = ["examples/raycast_demo.rs", "examples/fractal_demo.rs", "examples/mandelbrot_demo.rs"]

for filepath in examples:
    with open(filepath, "r") as f:
        content = f.read()

    # Find run_windowed(FooApp::new().unwrap());
    match_app = re.search(r"run_windowed\((\w+)::new\(\)\.unwrap\(\)\);", content)
    if match_app:
        app_name = match_app.group(1)
        replacement = f"""match {app_name}::new() {{
        Ok(app) => run_windowed(app),
        Err(e) => abrash::platform::print_error_and_exit(&e),
    }}"""
        content = content.replace(match_app.group(0), replacement)

        with open(filepath, "w") as f:
            f.write(content)
