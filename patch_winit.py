import sys

filepath = "src/platform/winit.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace(
    "fn print_host_error_and_exit(err: &HostError) -> ! {",
    "pub fn print_error_and_exit(err: &HostError) -> ! {"
)
content = content.replace(
    "print_host_error_and_exit",
    "print_error_and_exit"
)

# Insert the hint
old_table = "        .add_row(vec![Cell::new(format!(\"{err}\")).fg(Color::Yellow)]);"
new_table = """        .add_row(vec![Cell::new(format!("{err}")).fg(Color::Yellow)]);

    let err_str = err.to_string();
    if err_str.contains("WAYLAND_DISPLAY") || err_str.contains("X11") || err_str.contains("DISPLAY") {
        table.add_row(vec![Cell::new("💡 Hint: No display server found. Are you in a headless environment?\\nTry running with a TUI backend: --features backend-tui").fg(Color::Cyan)]);
    }"""

content = content.replace(old_table, new_table)

with open(filepath, "w") as f:
    f.write(content)
