#[cfg(test)]
mod tests {
    use std::time::Instant;
    use abrash::obj_loader::load_obj;

    #[test]
    fn test_dos_bucket_collision() {
        let n = 20_000;
        let mut obj = String::new();
        obj.push_str("v 0.0 0.0 0.0\n");
        for i in 0..n {
            obj.push_str(&format!("vt {} {}\n", i as f32 / n as f32, 0.0));
        }

        for i in 0..n/3 {
            let idx1 = i * 3 + 1;
            let idx2 = i * 3 + 2;
            let idx3 = i * 3 + 3;
            obj.push_str(&format!("f 1/{} 1/{} 1/{}\n", idx1, idx2, idx3));
        }

        let start = Instant::now();
        let _ = load_obj(&obj).unwrap();
        let duration = start.elapsed();

        println!("Loaded {} vertices in {:?}", n, duration);

        // Fail if too slow (e.g. > 2 seconds for 20k is suspicious)
        // 20k linear should be instant (< 100ms).
        // 20k quadratic is 400M operations.
        assert!(duration.as_millis() < 2000, "Potential DoS: took too long");
    }
}
