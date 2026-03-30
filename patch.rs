
fn check_fastpath_vulnerability(
    len: i32,
    u_fix: i32,
    du_fix: i32,
    tex_w: i32,
) -> Result<(), String> {
    if len <= 0 {
        return Ok(());
    }

    let u_start_64 = i64::from(u_fix);
    let du_64 = i64::from(du_fix);
    let u_end_64 = u_start_64 + du_64 * i64::from(len - 1);

    let (u_min_64, u_max_64) = if du_64 >= 0 {
        (u_start_64, u_end_64)
    } else {
        (u_end_64, u_start_64)
    };

    let can_use_fast_path =
        u_min_64 >= 0 && (u_max_64 >> 16) < i64::from(tex_w) && u_max_64 <= i64::from(i32::MAX);

    if can_use_fast_path {
        let mut current_u_fix = u_fix;
        for _ in 0..len {
            let u = (current_u_fix >> 16) as usize;

            if u >= tex_w as usize {
                return Err(format!("OOB access detected! u: {u}, tex_w: {tex_w}"));
            }

            current_u_fix = current_u_fix.wrapping_add(du_fix);
        }
    }

    Ok(())
}

fn main() {
    let result = check_fastpath_vulnerability(4096, 2147418112, 16384, 32768);
    println!("{:?}", result);
}
