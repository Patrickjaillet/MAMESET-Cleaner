use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("usage: bench_dat_parse <path-to-dat>");
    let started = Instant::now();
    let entries = mameset_cleaner::core::dat_parser::parse_dat_file(std::path::Path::new(&path))
        .expect("failed to parse DAT");
    println!(
        "parsed {} machines in {:.3}s",
        entries.len(),
        started.elapsed().as_secs_f64()
    );

    // Simulates what an in-session cache hit costs: no re-parse, just an
    // owned clone of the already-parsed data (see ROADMAP5.md v4.3.0).
    let clone_started = Instant::now();
    let cloned = entries.clone();
    println!(
        "cloned {} machines (simulated cache hit) in {:.3}s",
        cloned.len(),
        clone_started.elapsed().as_secs_f64()
    );
}
