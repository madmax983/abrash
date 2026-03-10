import re

with open('Cargo.toml', 'r') as f:
    text = f.read()

# I may have accidentally messed up Cargo.toml, I will use git checkout HEAD -- Cargo.toml
