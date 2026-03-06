with open("src/experimental/kuwahara.rs", "r") as f:
    code = f.read()

# Fix the extra closure issue
code = code.replace(
    """        }
    });
}""",
    """    });
}"""
)

with open("src/experimental/kuwahara.rs", "w") as f:
    f.write(code)
