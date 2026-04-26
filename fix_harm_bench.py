import re

with open("benches/harmonograph_bench.rs", "r") as f:
    code = f.read()

code = code.replace("let mut h =", "let h =")

with open("benches/harmonograph_bench.rs", "w") as f:
    f.write(code)
