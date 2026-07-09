with open('benches/post_process.rs', 'r') as f:
    content = f.read()

content = content.replace(
'''criterion_group!(
    benches,
    benchmark_grayscale,
    benchmark_scanlines,
    benchmark_invert,
    benchmark_sepia,
    benchmark_chromatic_aberration,
    benchmark_bloom,
    benchmark_ssao,
    benchmark_box_blur_f32,
    benchmark_box_blur_horizontal,
    benchmark_sobel,
    benchmark_dof,
    benchmark_vignette,
    benchmark_color_adjust,
    benchmark_pixel_sort,
    benchmark_halftone,
);''',
'''criterion_group!(
    benches,
    benchmark_grayscale,
    benchmark_scanlines,
    benchmark_invert,
    benchmark_sepia,
    benchmark_chromatic_aberration,
    benchmark_bloom,
    benchmark_ssao,
    benchmark_box_blur_f32,
    benchmark_box_blur_horizontal,
    benchmark_sobel,
    benchmark_dof,
    benchmark_vignette,
    benchmark_color_adjust,
    benchmark_pixel_sort,
    benchmark_halftone,
    benchmark_exposure,
);'''
)

with open('benches/post_process.rs', 'w') as f:
    f.write(content)
