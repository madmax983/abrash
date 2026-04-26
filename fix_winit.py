import re

with open("src/platform/winit.rs", "r") as f:
    code = f.read()

code = code.replace("print_host_error_and_exit(HostError", "print_host_error_and_exit(&HostError")
code = code.replace("print_host_error_and_exit(err);", "print_host_error_and_exit(&err);")

with open("src/platform/winit.rs", "w") as f:
    f.write(code)
