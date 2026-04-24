with open('crates/abrash-render/src/post_process/filters.rs', 'r') as f:
    text = f.read()

lines = text.split('\n')
for i in range(len(lines)):
    if 'use `abrash_core::framebuffer::Framebuffer`;' in lines[i]:
        lines[i] = lines[i].replace('`abrash_core::framebuffer::Framebuffer`', 'abrash_core::framebuffer::Framebuffer')
    if 'let mut fb = Framebuffer::new(1, 1).unwrap();' in lines[i]:
        if not lines[i].startswith('    '):
            lines[i] = '    ' + lines[i].lstrip()
    if 'fb.clear(0xFFC0_C0C0); // Light Gray (192)' in lines[i]:
        if not lines[i].startswith('    '):
            lines[i] = '    ' + lines[i].lstrip()
    if 'apply_solarize(&mut fb, 127);' in lines[i]:
        if not lines[i].startswith('    '):
            lines[i] = '    ' + lines[i].lstrip()
    if 'assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F);' in lines[i]:
        if not lines[i].startswith('    '):
            lines[i] = '    ' + lines[i].lstrip()


with open('crates/abrash-render/src/post_process/filters.rs', 'w') as f:
    f.write('\n'.join(lines))
