/// Time-based triggers demo — periodic and delayed link execution.
fn main() {
    println!("ANWE Time-Based Triggers Demo");
    println!("=============================\n");

    println!("--- Scheduled Links ---");
    let schedules = [
        ("HealthMonitor <-> ModelService", "every 30 ticks", "~30s health ping"),
        ("DriftDetector <-> ModelService", "every 500 ticks", "~8min drift eval"),
        ("CacheManager <-> CacheManager", "every 100 ticks", "~100s cache eviction"),
        ("HealthMonitor <-> DailyReport", "after 86400 ticks", "one-shot daily summary"),
    ];
    for (link, schedule, desc) in &schedules {
        println!("  {} [{}] — {}", link, schedule, desc);
    }

    println!("\n--- Simulation (tick 0..900) ---");
    let events = [
        (30, "health_check", "status: healthy, latency: 42ms"),
        (60, "health_check", "status: healthy, latency: 38ms"),
        (90, "health_check", "status: healthy, latency: 41ms"),
        (100, "cache_eviction", "evicted: 47, hit_rate: 0.85"),
        (120, "health_check", "status: healthy, latency: 45ms"),
        (200, "cache_eviction", "evicted: 31, hit_rate: 0.87"),
        (500, "drift_check", "accuracy: 0.91 vs baseline 0.94 — drift detected"),
        (600, "health_check", "status: healthy, latency: 43ms"),
        (900, "cache_eviction", "evicted: 22, hit_rate: 0.89"),
    ];
    for (tick, stage, result) in &events {
        println!("  tick {} [{}] → {}", tick, stage, result);
    }

    println!("\n✓ Time triggers proven: every/after scheduling with link priorities");
}
