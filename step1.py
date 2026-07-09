with open('crates/abrash-render/src/post_process/filters.rs', 'a') as f:
    f.write('''
#[test]
fn test_apply_exposure() {
    let mut fb = Framebuffer::new(2, 2).unwrap();
    fb.as_mut_slice()[0] = 0xFF00_0000 | (100 << 16) | (100 << 8) | 100;
    apply_exposure(&mut fb, 2.0);
    assert_eq!(fb.as_mut_slice()[0], 0xFF00_0000 | (200 << 16) | (200 << 8) | 200);
}
''')

with open('benches/post_process.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'benchmark_color_adjust,\n);',
    'benchmark_color_adjust,\n    benchmark_exposure,\n);'
)

content = content.replace(
    'fn benchmark_color_adjust(c: &mut Criterion) {',
    'fn benchmark_exposure(c: &mut Criterion) {\n    let mut fb = Framebuffer::new(1920, 1080).unwrap();\n    c.bench_function("apply_exposure 1080p", |b| {\n        b.iter(|| {\n            post_process::filters::apply_exposure(black_box(&mut fb), black_box(2.0));\n        });\n    });\n}\n\nfn benchmark_color_adjust(c: &mut Criterion) {'
)

with open('benches/post_process.rs', 'w') as f:
    f.write(content)
