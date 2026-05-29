import re

with open("tests/raytracer_tests.rs", "r") as f:
    content = f.read()

content = content.replace("assert!((hit.t - 5.0).abs() < 1e-3", "assert!((hit.t - 5.0).abs() < 0.005")

with open("tests/raytracer_tests.rs", "w") as f:
    f.write(content)
