import os
import sys

def process_file(filepath):
    with open(filepath, "r") as f:
        content = f.read()

    # We want to catch instances of run_windowed(SomeApp::new().unwrap())
    # and replace them with a match statement that calls print_error_and_exit.
    # To do this safely, we will specifically target the main function in a few examples.

    if "examples/cube_3d.rs" in filepath:
        content = content.replace(
            "run_windowed(Cube3dApp::new().unwrap());",
            """match Cube3dApp::new() {
        Ok(app) => run_windowed(app),
        Err(e) => abrash::platform::print_error_and_exit(&e),
    }"""
        )
    elif "examples/lit_cube.rs" in filepath:
        content = content.replace(
            "run_windowed(LitCubeApp::new().unwrap());",
            """match LitCubeApp::new() {
        Ok(app) => run_windowed(app),
        Err(e) => abrash::platform::print_error_and_exit(&e),
    }"""
        )

    with open(filepath, "w") as f:
        f.write(content)

process_file("examples/cube_3d.rs")
process_file("examples/lit_cube.rs")
