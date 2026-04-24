with open('crates/abrash-render/src/post_process/filters.rs', 'r') as f:
    text = f.read()

lines = text.split('\n')
for i in range(len(lines)):
    if 'let mut fb = Framebuffer::new(1, 1).unwrap();' in lines[i]:
        lines[i] = '/// let mut fb = Framebuffer::new(1, 1).unwrap();'
    if 'fb.clear(0xFFC0_C0C0); // Light Gray (192)' in lines[i]:
        lines[i] = '/// fb.clear(0xFFC0_C0C0); // Light Gray (192)'
    if 'apply_solarize(&mut fb, 127);' in lines[i]:
        lines[i] = '/// apply_solarize(&mut fb, 127);'
    if 'assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);' in lines[i]:
        lines[i] = '/// assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);'


with open('crates/abrash-render/src/post_process/filters.rs', 'w') as f:
    f.write('\n'.join(lines))
