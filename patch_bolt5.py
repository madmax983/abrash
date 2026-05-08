import re

file_path = "crates/abrash-render/src/post_process/filters.rs"
with open(file_path, "r") as f:
    content = f.read()

content = content.replace("/// `fb.set_pixel(0`, 0, 0xFF80_8080); // Mid-gray (128)", "/// `fb.set_pixel(0, 0, 0xFF80_8080);` // Mid-gray (128)")
content = content.replace("/// `fb.set_pixel(0`, 0, 0xFFFF_0000); // Red", "/// `fb.set_pixel(0, 0, 0xFFFF_0000);` // Red")
content = content.replace("/// `fb.set_pixel(0`, 0, 0xFF00_0000); // Black", "/// `fb.set_pixel(0, 0, 0xFF00_0000);` // Black")
content = content.replace("/// `assert_eq!(fb.get_pixel(0`, `0).unwrap()`, 0xFFFF_FFFF); // White", "/// `assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFF_FFFF);` // White")
content = content.replace("/// `fb.set_pixel(0`, 0, 0xFFFF_FFFF); // White", "/// `fb.set_pixel(0, 0, 0xFFFF_FFFF);` // White")
content = content.replace("/// `fb.set_pixel(50`, 50, 0xFFFF_FFFF); // White", "/// `fb.set_pixel(50, 50, 0xFFFF_FFFF);` // White")
content = content.replace("/// `fb.set_pixel(0`, 0, 0xFF80_8080); // Mid Gray (128)", "/// `fb.set_pixel(0, 0, 0xFF80_8080);` // Mid Gray (128)")

with open(file_path, "w") as f:
    f.write(content)

file_path = "crates/abrash-render/src/experimental/falling_sand.rs"
with open(file_path, "r") as f:
    content = f.read()

content = content.replace("0xFF00_0000).", "`0xFF00_0000`).")

with open(file_path, "w") as f:
    f.write(content)
