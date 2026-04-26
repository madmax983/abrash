import re

with open("benches/cel_shade_bench.rs", "r") as f:
    code = f.read()

code = code.replace("b.iter(|| {\n            apply_cel_shade(black_box(&mut fb), black_box(&zb), black_box(&config));\n        })", "b.iter(|| {\n            apply_cel_shade(black_box(&mut fb), black_box(&zb), black_box(&config));\n        });")

with open("benches/cel_shade_bench.rs", "w") as f:
    f.write(code)
