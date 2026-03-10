with open('Cargo.toml', 'r') as f:
    lines = f.readlines()

with open('Cargo.toml', 'w') as f:
    skip = False
    for line in lines:
        if line.strip() == '[[bench]]':
            skip = False
        if 'name = "displace_bench"' in line:
            skip = True

        if skip:
            continue

        f.write(line)
