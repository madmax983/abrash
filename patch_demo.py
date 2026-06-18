import re

with open("examples/interactive_paper_cutout.rs", "r") as f:
    code = f.read()

# Fix the floating point to u32
code = code.replace("FixedTimestep::new(60.0)", "FixedTimestep::new(60)")

# Fix the return type of run_windowed
code = code.replace("run_windowed(PaperCutoutDemoApp::new()?)?;", "run_windowed(PaperCutoutDemoApp::new()?);")

with open("examples/interactive_paper_cutout.rs", "w") as f:
    f.write(code)
