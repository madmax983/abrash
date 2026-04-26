with open("crates/abrash-render/src/experimental/voronoi.rs", 'r') as f:
    text = f.read()

part1 = text[:text.find('    if is_euclidean {\n')]
part3_start = text.find('}\n\n#[cfg(test)]\n')
part3 = text[part3_start:]

metrics = [
    {
        'cond': 'is_euclidean',
        'init': 'let mut min_dist_sq = f32::MAX;\n                let mut second_min_dist_sq = f32::MAX;\n                let mut closest_idx = 0;',
        'dist': 'let dist_sq = dx * dx + dy * dy;\n                    if dist_sq < min_dist_sq {\n                        second_min_dist_sq = min_dist_sq;\n                        min_dist_sq = dist_sq;\n                        closest_idx = i;\n                    } else if dist_sq < second_min_dist_sq {\n                        second_min_dist_sq = dist_sq;\n                    }',
        'border': 'let min_dist = min_dist_sq.sqrt();\n                    let second_min_dist = second_min_dist_sq.sqrt();\n                    let diff = (second_min_dist - min_dist).abs();\n                    if diff <= config.border_thickness {\n                        *pixel = config.border_color;\n                        continue;\n                    }',
        'init_fast': 'let mut min_dist_sq = f32::MAX;\n                let mut closest_idx = 0;',
        'dist_fast': 'let dist_sq = dx * dx + dy * dy;\n                    if dist_sq < min_dist_sq {\n                        min_dist_sq = dist_sq;\n                        closest_idx = i;\n                    }'
    },
    {
        'cond': 'is_manhattan',
        'init': 'let mut min_dist = f32::MAX;\n                let mut second_min_dist = f32::MAX;\n                let mut closest_idx = 0;',
        'dist': 'let dx = dx.abs();\n                    let dy = dy.abs();\n                    let dist = dx + dy;\n                    if dist < min_dist {\n                        second_min_dist = min_dist;\n                        min_dist = dist;\n                        closest_idx = i;\n                    } else if dist < second_min_dist {\n                        second_min_dist = dist;\n                    }',
        'border': 'let diff = (second_min_dist - min_dist).abs();\n                    if diff <= config.border_thickness {\n                        *pixel = config.border_color;\n                        continue;\n                    }',
        'init_fast': 'let mut min_dist = f32::MAX;\n                let mut closest_idx = 0;',
        'dist_fast': 'let dx = dx.abs();\n                    let dy = dy.abs();\n                    let dist = dx + dy;\n                    if dist < min_dist {\n                        min_dist = dist;\n                        closest_idx = i;\n                    }'
    },
    {
        'cond': 'is_cbrt',
        'init': 'let mut min_dist = f32::MAX;\n                let mut second_min_dist = f32::MAX;\n                let mut closest_idx = 0;',
        'dist': 'let dx = dx.abs();\n                    let dy = dy.abs();\n                    let dist = (dx * dx * dx + dy * dy * dy).cbrt();\n                    if dist < min_dist {\n                        second_min_dist = min_dist;\n                        min_dist = dist;\n                        closest_idx = i;\n                    } else if dist < second_min_dist {\n                        second_min_dist = dist;\n                    }',
        'border': 'let diff = (second_min_dist - min_dist).abs();\n                    if diff <= config.border_thickness {\n                        *pixel = config.border_color;\n                        continue;\n                    }',
        'init_fast': 'let mut min_dist = f32::MAX;\n                let mut closest_idx = 0;',
        'dist_fast': 'let dx = dx.abs();\n                    let dy = dy.abs();\n                    let dist = (dx * dx * dx + dy * dy * dy).cbrt();\n                    if dist < min_dist {\n                        min_dist = dist;\n                        closest_idx = i;\n                    }'
    },
    {
        'cond': 'is_sqrt',
        'init': 'let mut min_dist = f32::MAX;\n                let mut second_min_dist = f32::MAX;\n                let mut closest_idx = 0;',
        'dist': 'let dx = dx.abs();\n                    let dy = dy.abs();\n                    let x2 = dx * dx;\n                    let y2 = dy * dy;\n                    let dist = x2.hypot(y2).sqrt();\n                    if dist < min_dist {\n                        second_min_dist = min_dist;\n                        min_dist = dist;\n                        closest_idx = i;\n                    } else if dist < second_min_dist {\n                        second_min_dist = dist;\n                    }',
        'border': 'let diff = (second_min_dist - min_dist).abs();\n                    if diff <= config.border_thickness {\n                        *pixel = config.border_color;\n                        continue;\n                    }',
        'init_fast': 'let mut min_dist = f32::MAX;\n                let mut closest_idx = 0;',
        'dist_fast': 'let dx = dx.abs();\n                    let dy = dy.abs();\n                    let x2 = dx * dx;\n                    let y2 = dy * dy;\n                    let dist = x2.hypot(y2).sqrt();\n                    if dist < min_dist {\n                        min_dist = dist;\n                        closest_idx = i;\n                    }'
    },
    {
        'cond': 'else',
        'init': 'let mut min_dist = f32::MAX;\n                let mut second_min_dist = f32::MAX;\n                let mut closest_idx = 0;',
        'dist': 'let dx = dx.abs();\n                    let dy = dy.abs();\n                    let dist = (dx.powf(metric) + dy.powf(metric)).powf(inv_metric);\n                    if dist < min_dist {\n                        second_min_dist = min_dist;\n                        min_dist = dist;\n                        closest_idx = i;\n                    } else if dist < second_min_dist {\n                        second_min_dist = dist;\n                    }',
        'border': 'let diff = (second_min_dist - min_dist).abs();\n                    if diff <= config.border_thickness {\n                        *pixel = config.border_color;\n                        continue;\n                    }',
        'init_fast': 'let mut min_dist = f32::MAX;\n                let mut closest_idx = 0;',
        'dist_fast': 'let dx = dx.abs();\n                    let dy = dy.abs();\n                    let dist = (dx.powf(metric) + dy.powf(metric)).powf(inv_metric);\n                    if dist < min_dist {\n                        min_dist = dist;\n                        closest_idx = i;\n                    }'
    }
]

middle = ""

first = True
for m in metrics:
    if m['cond'] == 'else':
        middle += "    } else {\n"
    else:
        if first:
            middle += f"    if {m['cond']} {{\n"
            first = False
        else:
            middle += f"    }} else if {m['cond']} {{\n"

    middle += f"""        if config.border_thickness > 0.0 {{
            chunk_iter.enumerate().for_each(|(y, row)| {{
                let fy = y as f32;
                for (x, pixel) in row.iter_mut().enumerate() {{
                    let fx = x as f32;
                    {m['init']}

                    for (i, seed) in seeds.iter().enumerate() {{
                        let dx = fx - seed.x;
                        let dy = fy - seed.y;
                        {m['dist']}
                    }}

                    {m['border']}
                    *pixel = seed_colors[closest_idx];
                }}
            }});
        }} else {{
            chunk_iter.enumerate().for_each(|(y, row)| {{
                let fy = y as f32;
                for (x, pixel) in row.iter_mut().enumerate() {{
                    let fx = x as f32;
                    {m['init_fast']}

                    for (i, seed) in seeds.iter().enumerate() {{
                        let dx = fx - seed.x;
                        let dy = fy - seed.y;
                        {m['dist_fast']}
                    }}

                    *pixel = seed_colors[closest_idx];
                }}
            }});
        }}
"""

middle += "    }"

new_text = part1 + middle + part3

with open("crates/abrash-render/src/experimental/voronoi.rs", 'w') as f:
    f.write(new_text)
