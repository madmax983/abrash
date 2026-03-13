with open("src/experimental/lsystem.rs", "r") as f:
    text = f.read()
import re
text = re.sub(r'/// Bolt Performance Optimization:.*?\n\s*///.*?\n\s*///.*?\n\s*///.*?\n\s*///.*?\n', '', text)
with open("src/experimental/lsystem.rs", "w") as f:
    f.write(text)
