import re

with open("tests/havoc_circle_overflow.rs", "r") as f:
    code = f.read()

code = code.replace("0xFFFFFFFF", "0xFFFF_FFFF")

with open("tests/havoc_circle_overflow.rs", "w") as f:
    f.write(code)
