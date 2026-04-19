#![cfg(feature = "nova")]
use abrash_render::experimental::arboretum::LSystem;

#[test]
fn test_havoc_arboretum_oom() {
    // 👹 Havoc: Intentionally tests an Out-Of-Memory (OOM) vulnerability by creating a
    // runaway L-System stack. The `arboretum` module limits string expansion to 100MB,
    // but forgets to limit the `Turtle` state stack.
    // By generating a string of exclusively `[` commands (each pushing a 56-byte Turtle),
    // we force the system to allocate ~5.6 GB of RAM, causing an OOM crash.
    let mut lsys = LSystem::new("[", 90.0, 1.0, 0.1);
    lsys.add_rule('[', "[[[[[[[[[["); // 10x growth per iteration

    // 8 iterations: 10^8 characters = 100,000,000 commands
    // This is exactly the limit of the string expansion (100MB), so it will succeed.
    // However, when `generate_mesh` interprets the string, it pushes 100,000,000 Turtles.
    // 100M * 56 bytes = 5.6 GB allocation -> OOM / Crash
    let res = lsys.generate_mesh(8);
    assert!(res.is_err(), "Expected an error to prevent OOM");
}
