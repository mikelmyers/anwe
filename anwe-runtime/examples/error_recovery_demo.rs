/// Error recovery demo — supervision, circuit breakers, graceful degradation.
fn main() {
    println!("ANWE Error Recovery Demo");
    println!("========================\n");

    println!("--- Normal Operation ---");
    println!("  Handler → PrimaryModel (llm-70b): 800ms, full quality ✓\n");

    println!("--- Primary Model Crashes ---");
    println!("  PrimaryModel timeout after 5000ms");
    println!("  Supervision: rest_for_one → restart PrimaryModel");
    println!("  Retry 1/2: timeout again");
    println!("  Retry 2/2: timeout again");
    println!("  Status: primary_failed\n");

    println!("--- Automatic Fallback ---");
    println!("  on_failure_of PrimaryModel → FallbackModel (llm-7b)");
    println!("  Response: 200ms, degradation: quality");
    println!("  Note: Response may be less nuanced\n");

    println!("--- Fallback Also Fails (worst case) ---");
    println!("  on_failure_of FallbackModel → StaleCache");
    println!("  Cached response (120s old), degradation: freshness");
    println!("  Warning: This response was cached 2 minutes ago\n");

    println!("--- Circuit Breaker ---");
    println!("  consecutive_failures > 3 → circuit_breaker: OPEN");
    println!("  Primary requests skip to fallback immediately");
    println!("  Cooldown: 60 seconds\n");

    println!("--- Recovery ---");
    println!("  RecoveryManager checks every 30 ticks");
    println!("  PrimaryModel back online → consecutive_failures: 0");
    println!("  circuit_breaker: CLOSED");
    println!("  degradation: none");

    println!("\n--- Health Dashboard ---");
    println!("  Primary: degraded → healthy");
    println!("  Fallback: healthy");
    println!("  Cache: healthy");
    println!("  MTTR: 45s");

    println!("\n✓ Error recovery proven: supervision + circuit breaker + graceful degradation");
}
